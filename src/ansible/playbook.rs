use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlaybookSecurityError {
    #[error("Playbook path cannot be empty")]
    EmptyPath,
    #[error("Directory traversal attempt detected in path: '{path}'")]
    DirectoryTraversal { path: String },
    #[error("Playbook '{path}' is outside trusted playbook root '{root}'")]
    PathEscapesRoot { path: String, root: String },
    #[error("Invalid playbook extension in '{path}'; only .yml and .yaml are permitted")]
    InvalidExtension { path: String },
    #[error("Playbook '{name}' is not in the approved playbook whitelist")]
    NotApproved { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlaybookType {
    /// Read-only playbook (audits, diagnostics, backup, verification)
    ReadOnly,
    /// Potentially mutating playbook (configuration changes, remediation)
    Write,
}

impl PlaybookType {
    pub fn is_write(&self) -> bool {
        matches!(self, PlaybookType::Write)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybookType::ReadOnly => "Read-Only (Non-destructive)",
            PlaybookType::Write => "Write (Configuration Mutating)",
        }
    }
}

/// Metadata for an approved playbook in the whitelist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedPlaybook {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub playbook_type: PlaybookType,
    pub description: String,
}

/// Registry managing trusted playbook roots and whitelist enforcement.
#[derive(Debug, Clone)]
pub struct PlaybookRegistry {
    pub trusted_root: PathBuf,
    pub approved_playbooks: Vec<ApprovedPlaybook>,
}

impl Default for PlaybookRegistry {
    fn default() -> Self {
        Self::with_default_catalog(PathBuf::from("/etc/idf/playbooks"))
    }
}

impl PlaybookRegistry {
    pub fn new(trusted_root: impl Into<PathBuf>) -> Self {
        Self {
            trusted_root: trusted_root.into(),
            approved_playbooks: Vec::new(),
        }
    }

    pub fn with_default_catalog(trusted_root: impl Into<PathBuf>) -> Self {
        let mut reg = Self::new(trusted_root);
        reg.approved_playbooks = vec![
            ApprovedPlaybook {
                id: "pb-backup-cisco".to_string(),
                name: "Backup Cisco Switch Configurations".to_string(),
                filename: "backup_cisco_config.yml".to_string(),
                playbook_type: PlaybookType::ReadOnly,
                description: "Captures running-config from Cisco Catalyst switches and stores audit copies".to_string(),
            },
            ApprovedPlaybook {
                id: "pb-verify-interfaces".to_string(),
                name: "Verify Interface & Trunk States".to_string(),
                filename: "verify_interfaces.yml".to_string(),
                playbook_type: PlaybookType::ReadOnly,
                description: "Validates port operational states, errors, and VLAN associations".to_string(),
            },
            ApprovedPlaybook {
                id: "pb-collect-diagnostics".to_string(),
                name: "Collect Switch Telemetry Diagnostics".to_string(),
                filename: "collect_diagnostics.yml".to_string(),
                playbook_type: PlaybookType::ReadOnly,
                description: "Gathers CPU, memory, PSU, temperature, and PoE inline telemetry".to_string(),
            },
            ApprovedPlaybook {
                id: "pb-update-vlan".to_string(),
                name: "Update Access VLAN Membership".to_string(),
                filename: "update_access_vlan.yml".to_string(),
                playbook_type: PlaybookType::Write,
                description: "Configures access VLAN on specified switch interfaces (Requires Approval Gate)".to_string(),
            },
            ApprovedPlaybook {
                id: "pb-remediate-errdisable".to_string(),
                name: "Remediate ErrDisabled Port Bounce".to_string(),
                filename: "remediate_errdisabled_port.yml".to_string(),
                playbook_type: PlaybookType::Write,
                description: "Executes shutdown / no shutdown bounce on resolved err-disabled ports (Requires Approval Gate)".to_string(),
            },
            ApprovedPlaybook {
                id: "pb-sync-ntp-syslog".to_string(),
                name: "Synchronize NTP & Syslog Logging".to_string(),
                filename: "sync_ntp_syslog.yml".to_string(),
                playbook_type: PlaybookType::Write,
                description: "Deploys standard campus NTP server and Syslog collector IPs (Requires Approval Gate)".to_string(),
            },
        ];
        reg
    }

    /// Validates a requested playbook filename, ID, or relative path against security constraints.
    pub fn validate_path(&self, requested: &str) -> Result<PathBuf, PlaybookSecurityError> {
        let trimmed = requested.trim();
        if trimmed.is_empty() {
            return Err(PlaybookSecurityError::EmptyPath);
        }

        // 1. Disallow directory traversal markers
        if trimmed.contains("..") || trimmed.contains('\\') || trimmed.contains('\0') {
            return Err(PlaybookSecurityError::DirectoryTraversal {
                path: trimmed.to_string(),
            });
        }

        // 2. If it directly matches an approved playbook by ID, filename, or name
        if let Some(approved_pb) = self
            .approved_playbooks
            .iter()
            .find(|pb| pb.id == trimmed || pb.filename == trimmed || pb.name == trimmed)
        {
            let resolved = self.trusted_root.join(&approved_pb.filename);
            if !resolved.starts_with(&self.trusted_root) {
                return Err(PlaybookSecurityError::PathEscapesRoot {
                    path: resolved.display().to_string(),
                    root: self.trusted_root.display().to_string(),
                });
            }
            return Ok(resolved);
        }

        // 3. Otherwise validate file extension
        let path = Path::new(trimmed);
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yml") | Some("yaml") => {}
            _ => {
                return Err(PlaybookSecurityError::InvalidExtension {
                    path: trimmed.to_string(),
                });
            }
        }

        Err(PlaybookSecurityError::NotApproved {
            name: trimmed.to_string(),
        })
    }

    /// Retrieve playbook metadata by filename or ID.
    pub fn find_playbook(&self, id_or_filename: &str) -> Option<&ApprovedPlaybook> {
        self.approved_playbooks.iter().find(|pb| {
            pb.id == id_or_filename || pb.filename == id_or_filename || pb.name == id_or_filename
        })
    }
}
