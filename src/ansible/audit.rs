use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::ansible::approval::ApprovalStatus;
use crate::ansible::playbook::PlaybookType;
use crate::ansible::status::{DeviceJobStatus, JobState};
use crate::ansible::verification::VerificationReport;

/// An immutable, secret-redacted historical record of an Ansible automation job.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub job_id: String,
    pub timestamp: String,
    pub operator: String,
    pub playbook_id: String,
    pub playbook_name: String,
    pub playbook_type: PlaybookType,
    pub targets: Vec<String>,
    pub is_dry_run: bool,
    pub approval_status: ApprovalStatus,
    pub state: JobState,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub sanitized_command: String,
    pub per_device_status: HashMap<String, DeviceJobStatus>,
    pub verification: Option<VerificationReport>,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
}

impl AuditEntry {
    pub fn is_successful(&self) -> bool {
        self.state == JobState::Completed && self.exit_code == Some(0)
    }

    pub fn device_count(&self) -> usize {
        self.targets.len()
    }
}

/// Thread-safe in-memory audit log managing historical records of automation operations.
#[derive(Debug, Clone)]
pub struct AuditTrail {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a completed or updated audit entry.
    pub fn record(&self, entry: AuditEntry) {
        if let Ok(mut lock) = self.entries.write() {
            // Update if existing job_id, or append new
            if let Some(pos) = lock.iter().position(|e| e.job_id == entry.job_id) {
                lock[pos] = entry;
            } else {
                lock.push(entry);
            }
        }
    }

    /// Retrieve all recorded audit entries in chronological order.
    pub fn all_entries(&self) -> Vec<AuditEntry> {
        self.entries.read().map(|l| l.clone()).unwrap_or_default()
    }

    /// Retrieve an audit entry by its unique job identifier.
    pub fn find_by_job_id(&self, job_id: &str) -> Option<AuditEntry> {
        self.entries
            .read()
            .ok()?
            .iter()
            .find(|e| e.job_id == job_id)
            .cloned()
    }

    /// Filter audit entries associated with a specific switch or hostname.
    pub fn find_by_device(&self, device: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .map(|l| {
                l.iter()
                    .filter(|e| e.targets.iter().any(|t| t == device))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.entries.read().map(|l| l.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
