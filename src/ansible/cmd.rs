use crate::provider::sanitizer::LogSanitizer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Safe command builder constructing strict argument arrays for ansible-playbook
/// execution without shell interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleCommandBuilder {
    pub binary: PathBuf,
    pub playbook_path: PathBuf,
    pub inventory_path: PathBuf,
    pub limit: Option<String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub verbosity: u8,
    pub extra_vars: HashMap<String, String>,
}

impl AnsibleCommandBuilder {
    pub fn new(playbook_path: impl AsRef<Path>, inventory_path: impl AsRef<Path>) -> Self {
        Self {
            binary: PathBuf::from("ansible-playbook"),
            playbook_path: playbook_path.as_ref().to_path_buf(),
            inventory_path: inventory_path.as_ref().to_path_buf(),
            limit: None,
            check_mode: false,
            diff_mode: false,
            verbosity: 1,
            extra_vars: HashMap::new(),
        }
    }

    pub fn with_binary(mut self, binary: impl AsRef<Path>) -> Self {
        self.binary = binary.as_ref().to_path_buf();
        self
    }

    pub fn with_limit(mut self, limit: impl Into<String>) -> Self {
        self.limit = Some(limit.into());
        self
    }

    pub fn with_check_mode(mut self, check: bool) -> Self {
        self.check_mode = check;
        self
    }

    pub fn with_diff_mode(mut self, diff: bool) -> Self {
        self.diff_mode = diff;
        self
    }

    pub fn with_verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    pub fn with_extra_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_vars.insert(key.into(), value.into());
        self
    }

    /// Construct a pure vector of arguments for direct `std::process::Command::args` execution.
    /// This eliminates any shell quoting or interpolation vulnerabilities.
    pub fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 1. Inventory specification
        args.push("-i".to_string());
        args.push(self.inventory_path.display().to_string());

        // 2. Target limitation
        if let Some(ref limit) = self.limit
            && !limit.trim().is_empty()
        {
            args.push("--limit".to_string());
            args.push(limit.clone());
        }

        // 3. Dry-run / Check & Diff modes
        if self.check_mode {
            args.push("--check".to_string());
        }
        if self.diff_mode {
            args.push("--diff".to_string());
        }

        // 4. Verbosity level
        if self.verbosity == 1 {
            args.push("-v".to_string());
        } else if self.verbosity == 2 {
            args.push("-vv".to_string());
        } else if self.verbosity >= 3 {
            args.push("-vvv".to_string());
        }

        // 5. Extra variables (JSON formatted if present)
        if !self.extra_vars.is_empty()
            && let Ok(json_str) = serde_json::to_string(&self.extra_vars)
        {
            args.push("--extra-vars".to_string());
            args.push(json_str);
        }

        // 6. Playbook target path (last positional argument)
        args.push(self.playbook_path.display().to_string());

        args
    }

    /// Construct a human-readable, secret-sanitized command representation for audit logs.
    pub fn build_sanitized_display(&self) -> String {
        let args = self.build_args();
        let binary_str = self.binary.display().to_string();
        let full_command = format!("{binary_str} {}", args.join(" "));
        LogSanitizer::sanitize(&full_command)
    }
}
