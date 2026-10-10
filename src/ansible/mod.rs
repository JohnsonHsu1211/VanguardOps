pub mod approval;
pub mod audit;
pub mod cmd;
pub mod playbook;
pub mod runner;
pub mod status;
pub mod target;
pub mod verification;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub use approval::{ApprovalError, ApprovalGate, ApprovalStatus, WriteProtectionPolicy};
pub use audit::{AuditEntry, AuditTrail};
pub use cmd::AnsibleCommandBuilder;
pub use playbook::{ApprovedPlaybook, PlaybookRegistry, PlaybookSecurityError, PlaybookType};
pub use runner::{
    MockSubprocessRunner, ProcessSubprocessRunner, SubprocessError, SubprocessOutput,
    SubprocessRunner,
};
pub use status::{DeviceExecutionState, DeviceJobStatus, JobState, StatusParser};
pub use target::{TargetSelection, TargetValidationError, TargetValidator, ValidatedTargets};
pub use verification::{
    PostChangeVerifier, VerificationCheck, VerificationReport, VerificationStatus,
};

use crate::model::NetworkInfrastructure;
use crate::provider::sanitizer::LogSanitizer;

#[derive(Debug, thiserror::Error)]
pub enum AnsibleError {
    #[error("Target validation error: {0}")]
    Target(#[from] TargetValidationError),

    #[error("Playbook security error: {0}")]
    Playbook(#[from] PlaybookSecurityError),

    #[error("Approval gate error: {0}")]
    Approval(#[from] ApprovalError),

    #[error("Subprocess runner error: {0}")]
    Subprocess(#[from] SubprocessError),

    #[error("Job '{job_id}' execution failed with exit code {exit_code}: {details}")]
    ExecutionFailed {
        job_id: String,
        exit_code: i32,
        details: String,
    },

    #[error("Job '{job_id}' timed out after {duration:?}")]
    Timeout { job_id: String, duration: Duration },

    #[error("Job '{0}' not found or invalid")]
    JobNotFound(String),
}

/// Request to create and execute or stage an Ansible automation job.
#[derive(Debug, Clone)]
pub struct JobRequest {
    pub playbook_id: String,
    pub target_selection: TargetSelection,
    pub operator: String,
    pub is_dry_run: bool,
    pub justification: Option<String>,
    pub extra_vars: HashMap<String, String>,
    pub timeout: Option<Duration>,
}

impl JobRequest {
    pub fn dry_run(
        playbook_id: impl Into<String>,
        target: TargetSelection,
        operator: impl Into<String>,
    ) -> Self {
        Self {
            playbook_id: playbook_id.into(),
            target_selection: target,
            operator: operator.into(),
            is_dry_run: true,
            justification: None,
            extra_vars: HashMap::new(),
            timeout: None,
        }
    }

    pub fn live(
        playbook_id: impl Into<String>,
        target: TargetSelection,
        operator: impl Into<String>,
        justification: impl Into<String>,
    ) -> Self {
        Self {
            playbook_id: playbook_id.into(),
            target_selection: target,
            operator: operator.into(),
            is_dry_run: false,
            justification: Some(justification.into()),
            extra_vars: HashMap::new(),
            timeout: None,
        }
    }
}

/// Dedicated Subprocess Adapter orchestrating Ansible playbook automation
/// with strict security boundaries, approval gating, and audit logging.
pub struct AnsibleAdapter {
    pub runner: Arc<dyn SubprocessRunner>,
    pub playbook_registry: PlaybookRegistry,
    pub approval_gate: ApprovalGate,
    pub audit_trail: AuditTrail,
    pub inventory_path: PathBuf,
    pub default_timeout: Duration,
    job_counter: std::sync::atomic::AtomicU64,
}

impl AnsibleAdapter {
    pub fn new(
        runner: Arc<dyn SubprocessRunner>,
        playbook_registry: PlaybookRegistry,
        approval_gate: ApprovalGate,
        inventory_path: impl Into<PathBuf>,
        default_timeout: Duration,
    ) -> Self {
        Self {
            runner,
            playbook_registry,
            approval_gate,
            audit_trail: AuditTrail::new(),
            inventory_path: inventory_path.into(),
            default_timeout,
            job_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Safe default mock adapter initialized with simulated runner and write operations disabled.
    pub fn new_mock(trusted_root: impl Into<PathBuf>) -> Self {
        let runner = Arc::new(MockSubprocessRunner::new());
        let registry = PlaybookRegistry::with_default_catalog(trusted_root);
        let gate = ApprovalGate::new(WriteProtectionPolicy::default()); // Write disabled by default
        Self::new(
            runner,
            registry,
            gate,
            PathBuf::from("/etc/idf/inventory/hosts.ini"),
            Duration::from_secs(60),
        )
    }

    fn next_job_id(&self) -> String {
        let num = self
            .job_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("JOB-20261010-{num:03}")
    }

    /// Submit and process an automation job request.
    /// If write operations require approval, stages the job as `AwaitingApproval`.
    /// If read-only or dry-run, executes immediately through the subprocess adapter.
    pub fn submit_job(
        &self,
        req: JobRequest,
        net: &NetworkInfrastructure,
    ) -> Result<AuditEntry, AnsibleError> {
        let job_id = self.next_job_id();

        // 1. Target validation
        let validated_targets = TargetValidator::validate(&req.target_selection, net, false)?;

        // 2. Playbook validation & path verification
        let playbook_path = self.playbook_registry.validate_path(&req.playbook_id)?;
        let playbook = self
            .playbook_registry
            .find_playbook(&req.playbook_id)
            .ok_or_else(|| PlaybookSecurityError::NotApproved {
                name: req.playbook_id.clone(),
            })?
            .clone();

        let initial_approval = if !playbook.playbook_type.is_write() || req.is_dry_run {
            ApprovalStatus::NotRequired
        } else {
            ApprovalStatus::Pending
        };

        // 3. Evaluate approval gate
        let gate_res = self.approval_gate.evaluate(
            &job_id,
            playbook.playbook_type,
            req.is_dry_run,
            &req.operator,
            &initial_approval,
        );

        match gate_res {
            Ok(()) => {
                // Execute immediately (Read-only, Dry-Run, or Pre-approved)
                self.execute_job_internal(
                    &job_id,
                    &playbook,
                    &playbook_path,
                    &validated_targets,
                    &req,
                    initial_approval,
                    net,
                )
            }
            Err(ApprovalError::ApprovalRequired { .. }) => {
                // Stage in audit trail as AwaitingApproval
                let cmd_builder = AnsibleCommandBuilder::new(&playbook_path, &self.inventory_path)
                    .with_limit(&validated_targets.limit_arg)
                    .with_check_mode(req.is_dry_run);

                let entry = AuditEntry {
                    job_id: job_id.clone(),
                    timestamp: "2026-10-10 10:15:00 UTC".to_string(),
                    operator: req.operator.clone(),
                    playbook_id: playbook.id.clone(),
                    playbook_name: playbook.name.clone(),
                    playbook_type: playbook.playbook_type,
                    targets: validated_targets.hostnames.clone(),
                    is_dry_run: req.is_dry_run,
                    approval_status: initial_approval,
                    state: JobState::AwaitingApproval,
                    exit_code: None,
                    duration: Duration::ZERO,
                    sanitized_command: cmd_builder.build_sanitized_display(),
                    per_device_status: HashMap::new(),
                    verification: None,
                    stdout_excerpt: String::new(),
                    stderr_excerpt: String::new(),
                };

                self.audit_trail.record(entry.clone());
                Ok(entry)
            }
            Err(e) => Err(AnsibleError::Approval(e)),
        }
    }

    /// Explicitly approve a staged job and proceed with subprocess execution.
    pub fn approve_and_execute_job(
        &self,
        job_id: &str,
        approver: &str,
        notes: &str,
        net: &NetworkInfrastructure,
    ) -> Result<AuditEntry, AnsibleError> {
        let existing = self
            .audit_trail
            .find_by_job_id(job_id)
            .ok_or_else(|| AnsibleError::JobNotFound(job_id.to_string()))?;

        if existing.state != JobState::AwaitingApproval {
            return Err(AnsibleError::Approval(ApprovalError::ApprovalRequired {
                job_id: job_id.to_string(),
                current_state: existing.state.as_str().to_string(),
            }));
        }

        let playbook = self
            .playbook_registry
            .find_playbook(&existing.playbook_id)
            .ok_or_else(|| PlaybookSecurityError::NotApproved {
                name: existing.playbook_id.clone(),
            })?
            .clone();

        let playbook_path = self
            .playbook_registry
            .validate_path(&existing.playbook_id)?;

        let targets = TargetValidator::validate(
            &TargetSelection::Multiple(existing.targets.clone()),
            net,
            false,
        )?;

        let approval = ApprovalStatus::Approved {
            approver: approver.to_string(),
            approved_at: "2026-10-10 10:16:00 UTC".to_string(),
            notes: notes.to_string(),
        };

        // Re-evaluate approval gate with explicit approval decision
        self.approval_gate.evaluate(
            job_id,
            playbook.playbook_type,
            existing.is_dry_run,
            &existing.operator,
            &approval,
        )?;

        let req = JobRequest {
            playbook_id: existing.playbook_id,
            target_selection: TargetSelection::Multiple(existing.targets),
            operator: existing.operator,
            is_dry_run: existing.is_dry_run,
            justification: None,
            extra_vars: HashMap::new(),
            timeout: Some(self.default_timeout),
        };

        self.execute_job_internal(
            job_id,
            &playbook,
            &playbook_path,
            &targets,
            &req,
            approval,
            net,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_job_internal(
        &self,
        job_id: &str,
        playbook: &ApprovedPlaybook,
        playbook_path: &PathBuf,
        targets: &ValidatedTargets,
        req: &JobRequest,
        approval: ApprovalStatus,
        net: &NetworkInfrastructure,
    ) -> Result<AuditEntry, AnsibleError> {
        let timeout = req.timeout.unwrap_or(self.default_timeout);

        // 1. Build argument-array command without shell interpolation
        let mut cmd_builder = AnsibleCommandBuilder::new(playbook_path, &self.inventory_path)
            .with_limit(&targets.limit_arg)
            .with_check_mode(req.is_dry_run)
            .with_diff_mode(req.is_dry_run);

        for (k, v) in &req.extra_vars {
            cmd_builder = cmd_builder.with_extra_var(k, v);
        }

        let args = cmd_builder.build_args();
        let sanitized_cmd = cmd_builder.build_sanitized_display();

        // 2. Execute via dedicated subprocess runner
        let env = HashMap::new();
        let sub_out = self.runner.run(&cmd_builder.binary, &args, &env, timeout)?;

        if sub_out.timed_out {
            let entry = AuditEntry {
                job_id: job_id.to_string(),
                timestamp: "2026-10-10 10:15:00 UTC".to_string(),
                operator: req.operator.clone(),
                playbook_id: playbook.id.clone(),
                playbook_name: playbook.name.clone(),
                playbook_type: playbook.playbook_type,
                targets: targets.hostnames.clone(),
                is_dry_run: req.is_dry_run,
                approval_status: approval,
                state: JobState::TimedOut,
                exit_code: Some(sub_out.exit_code),
                duration: sub_out.duration,
                sanitized_command: sanitized_cmd,
                per_device_status: HashMap::new(),
                verification: None,
                stdout_excerpt: String::new(),
                stderr_excerpt: sub_out.stderr,
            };
            self.audit_trail.record(entry.clone());
            return Err(AnsibleError::Timeout {
                job_id: job_id.to_string(),
                duration: timeout,
            });
        }

        // 3. Parse per-device status from PLAY RECAP
        let per_device = StatusParser::parse_play_recap(&sub_out.stdout);

        // 4. Run post-change verification
        let target_switches: Vec<_> = targets
            .hostnames
            .iter()
            .filter_map(|h| {
                net.closets
                    .iter()
                    .flat_map(|c| &c.racks)
                    .flat_map(|r| &r.switches)
                    .find(|s| s.hostname == *h)
                    .cloned()
            })
            .collect();

        let verification_report =
            PostChangeVerifier::verify(&target_switches, &per_device, req.is_dry_run);

        let job_state = if sub_out.exit_code == 0 {
            JobState::Completed
        } else {
            JobState::Failed
        };

        let sanitized_stdout = LogSanitizer::sanitize(&sub_out.stdout);
        let sanitized_stderr = LogSanitizer::sanitize(&sub_out.stderr);

        let entry = AuditEntry {
            job_id: job_id.to_string(),
            timestamp: "2026-10-10 10:15:00 UTC".to_string(),
            operator: req.operator.clone(),
            playbook_id: playbook.id.clone(),
            playbook_name: playbook.name.clone(),
            playbook_type: playbook.playbook_type,
            targets: targets.hostnames.clone(),
            is_dry_run: req.is_dry_run,
            approval_status: approval,
            state: job_state,
            exit_code: Some(sub_out.exit_code),
            duration: sub_out.duration,
            sanitized_command: sanitized_cmd,
            per_device_status: per_device,
            verification: Some(verification_report),
            stdout_excerpt: sanitized_stdout
                .lines()
                .take(20)
                .collect::<Vec<_>>()
                .join("\n"),
            stderr_excerpt: sanitized_stderr,
        };

        self.audit_trail.record(entry.clone());

        if sub_out.exit_code != 0 {
            return Err(AnsibleError::ExecutionFailed {
                job_id: job_id.to_string(),
                exit_code: sub_out.exit_code,
                details: entry.stderr_excerpt.clone(),
            });
        }

        Ok(entry)
    }
}
