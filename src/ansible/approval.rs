use crate::ansible::playbook::PlaybookType;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApprovalError {
    #[error(
        "Write operations are globally disabled by default. Enable via policy to allow configuration changes."
    )]
    WriteOperationsDisabled,
    #[error(
        "Job '{job_id}' requires explicit approval before write execution (Current state: {current_state})"
    )]
    ApprovalRequired {
        job_id: String,
        current_state: String,
    },
    #[error(
        "Two-person rule violation: approver '{approver}' cannot be the same as operator '{operator}'"
    )]
    TwoPersonRuleViolation { approver: String, operator: String },
    #[error("Job '{job_id}' was rejected: {reason}")]
    JobRejected { job_id: String, reason: String },
}

/// Global safety policy enforcing write operation restrictions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteProtectionPolicy {
    /// Strict master switch: write operations are disabled by default
    pub write_operations_enabled: bool,
    /// Require a distinct peer reviewer to approve write operations
    pub require_two_person_rule: bool,
}

/// Status of an approval gate request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalStatus {
    NotRequired,
    Pending,
    Approved {
        approver: String,
        approved_at: String,
        notes: String,
    },
    Rejected {
        approver: String,
        rejected_at: String,
        reason: String,
    },
}

impl ApprovalStatus {
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            ApprovalStatus::Approved { .. } | ApprovalStatus::NotRequired
        )
    }

    pub fn approver(&self) -> Option<&str> {
        match self {
            ApprovalStatus::Approved { approver, .. } => Some(approver),
            _ => None,
        }
    }
}

/// Explicit approval gate that validates authorization before playbook execution.
#[derive(Debug, Clone)]
pub struct ApprovalGate {
    pub policy: WriteProtectionPolicy,
}

impl ApprovalGate {
    pub fn new(policy: WriteProtectionPolicy) -> Self {
        Self { policy }
    }

    /// Evaluates if a job is authorized to proceed to execution.
    pub fn evaluate(
        &self,
        job_id: &str,
        playbook_type: PlaybookType,
        is_dry_run: bool,
        operator: &str,
        approval: &ApprovalStatus,
    ) -> Result<(), ApprovalError> {
        // 1. Read-only playbooks and dry-runs do not modify state
        if !playbook_type.is_write() || is_dry_run {
            return Ok(());
        }

        // 2. Enforce global write-operations lock
        if !self.policy.write_operations_enabled {
            return Err(ApprovalError::WriteOperationsDisabled);
        }

        // 3. Evaluate explicit approval state
        match approval {
            ApprovalStatus::Approved { approver, .. } => {
                if self.policy.require_two_person_rule && approver.eq_ignore_ascii_case(operator) {
                    return Err(ApprovalError::TwoPersonRuleViolation {
                        approver: approver.clone(),
                        operator: operator.to_string(),
                    });
                }
                Ok(())
            }
            ApprovalStatus::Rejected { reason, .. } => Err(ApprovalError::JobRejected {
                job_id: job_id.to_string(),
                reason: reason.clone(),
            }),
            ApprovalStatus::Pending => Err(ApprovalError::ApprovalRequired {
                job_id: job_id.to_string(),
                current_state: "PENDING_APPROVAL".to_string(),
            }),
            ApprovalStatus::NotRequired => Err(ApprovalError::ApprovalRequired {
                job_id: job_id.to_string(),
                current_state: "APPROVAL_MISSING".to_string(),
            }),
        }
    }
}
