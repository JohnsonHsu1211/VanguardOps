use crate::ansible::status::{DeviceExecutionState, DeviceJobStatus};
use crate::model::Switch;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VerificationStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationStatus::Passed => "PASSED",
            VerificationStatus::Warning => "WARNING",
            VerificationStatus::Failed => "FAILED",
            VerificationStatus::Skipped => "SKIPPED",
        }
    }
}

/// An individual health or state verification check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub device: String,
    pub passed: bool,
    pub details: String,
}

/// Consolidated outcome of post-change verification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationReport {
    pub status: VerificationStatus,
    pub checks: Vec<VerificationCheck>,
    pub summary: String,
}

impl VerificationReport {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Skipped,
            checks: Vec::new(),
            summary: reason.into(),
        }
    }
}

/// Verifier assessing switch health and operational criteria following changes.
pub struct PostChangeVerifier;

impl PostChangeVerifier {
    /// Evaluate post-change telemetry and execution outcomes.
    pub fn verify(
        devices: &[Switch],
        per_device_status: &HashMap<String, DeviceJobStatus>,
        is_dry_run: bool,
    ) -> VerificationReport {
        if is_dry_run {
            return VerificationReport::skipped(
                "Dry-run / check mode: post-change verification skipped",
            );
        }

        let mut checks = Vec::new();
        let mut has_failure = false;
        let mut has_warning = false;

        for sw in devices {
            let host = &sw.hostname;
            let status = per_device_status.get(host);

            // 1. Check Ansible execution state
            let execution_ok = match status {
                Some(s) => {
                    s.state != DeviceExecutionState::Failed
                        && s.state != DeviceExecutionState::Unreachable
                }
                None => false,
            };

            checks.push(VerificationCheck {
                name: "Ansible Playbook Task Success".to_string(),
                device: host.clone(),
                passed: execution_ok,
                details: if execution_ok {
                    "All playbook tasks completed without unhandled errors".to_string()
                } else {
                    has_failure = true;
                    "Playbook tasks reported failures or host unreachable".to_string()
                },
            });

            // 2. Check Management Connectivity & Chassis Status
            let reachable = sw.status != crate::model::DeviceStatus::Offline;
            checks.push(VerificationCheck {
                name: "Management Reachability & Health".to_string(),
                device: host.clone(),
                passed: reachable,
                details: if reachable {
                    format!("Device is responsive, chassis state: {}", sw.status)
                } else {
                    has_failure = true;
                    "Device is offline or non-responsive after change".to_string()
                },
            });

            // 3. Check for Err-Disabled or Interface Drop Surges
            let err_count = sw.err_ports_count();
            let err_check_passed = err_count == 0;
            if err_count > 0 {
                has_warning = true;
            }

            checks.push(VerificationCheck {
                name: "Interface Health & Err-Disable Check".to_string(),
                device: host.clone(),
                passed: err_check_passed,
                details: if err_check_passed {
                    "No err-disabled ports detected on switch".to_string()
                } else {
                    format!("Attention: {err_count} err-disabled interface(s) detected")
                },
            });

            // 4. Environmental & Thermal Verification
            let temp_ok = sw.temp_celsius <= 50;
            if !temp_ok {
                has_warning = true;
            }
            checks.push(VerificationCheck {
                name: "Thermal & Environmental Check".to_string(),
                device: host.clone(),
                passed: temp_ok,
                details: format!("Internal chassis temperature: {}°C", sw.temp_celsius),
            });
        }

        let (status, summary) = if has_failure {
            (
                VerificationStatus::Failed,
                format!(
                    "Post-change verification failed on {} check(s)",
                    checks.iter().filter(|c| !c.passed).count()
                ),
            )
        } else if has_warning {
            (
                VerificationStatus::Warning,
                "Post-change verification passed with warnings (check details)".to_string(),
            )
        } else {
            (
                VerificationStatus::Passed,
                format!(
                    "All {} post-change verification checks passed successfully",
                    checks.len()
                ),
            )
        };

        VerificationReport {
            status,
            checks,
            summary,
        }
    }
}
