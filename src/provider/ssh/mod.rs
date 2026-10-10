pub mod host_key;
pub mod mock;

use crate::provider::DeviceTarget;
use crate::provider::credentials::CredentialError;
use crate::provider::ssh::host_key::HostKeyError;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error(
        "SECURITY VIOLATION: Command '{command}' is not an approved read-only command. Configuration and write commands are strictly forbidden."
    )]
    ReadOnlyViolation { command: String },

    #[error("SSH Host Key Error: {0}")]
    HostKeyError(#[from] HostKeyError),

    #[error("SSH Authentication failed for user '{username}' on {host}: {details}")]
    AuthFailed {
        username: String,
        host: String,
        details: String,
    },

    #[error("SSH Connection to {host}:{port} failed: {reason}")]
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },

    #[error("SSH operation timed out during {phase} after {duration:?}")]
    Timeout { phase: String, duration: Duration },

    #[error("SSH Command '{command}' exited with code {code}: {stderr}")]
    ExecutionFailed {
        command: String,
        code: i32,
        stderr: String,
    },

    #[error("Credential error: {0}")]
    CredentialError(#[from] CredentialError),

    #[error("Transport error: {0}")]
    Transport(String),
}

/// Output of a remote read-only command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(command: impl Into<String>, stdout: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
        }
    }
}

/// Strict validator that guarantees only read-only show/diagnostic commands can be executed.
pub struct ReadOnlyCommandValidator;

impl ReadOnlyCommandValidator {
    /// Whitelist of permitted command prefixes (case-insensitive)
    const ALLOWED_SHOW_PREFIXES: &'static [&'static str] = &[
        "show version",
        "show inventory",
        "show interfaces",
        "show environment",
        "show processes cpu",
        "show processes memory",
        "show power inline",
        "show switch",
        "show vlan",
        "show lldp neighbors",
        "show cdp neighbors",
        "show mac address-table",
        "show running-config | include",
        "ping",
    ];

    /// Forbidden keywords that MUST NEVER appear in commands
    const FORBIDDEN_KEYWORDS: &'static [&'static str] = &[
        "configure",
        "config",
        "conf t",
        "write",
        "erase",
        "copy",
        "delete",
        "reload",
        "reboot",
        "shutdown",
        "no shut",
        "no switchport",
        "set",
        "crypto",
        "clear",
        "debug",
        "boot",
        "format",
        "mkdir",
        "rmdir",
        "license",
    ];

    /// Validates whether a command string is an approved read-only operation.
    pub fn validate(command: &str) -> Result<(), SshError> {
        let trimmed = command.trim();
        let lower = trimmed.to_lowercase();

        if lower.is_empty() {
            return Err(SshError::ReadOnlyViolation {
                command: command.to_string(),
            });
        }

        // 1. Check for any forbidden destructive / configuration keyword
        for &forbidden in Self::FORBIDDEN_KEYWORDS {
            let re_pattern = format!(r"\b{}\b", regex::escape(forbidden));
            if let Ok(re) = regex::Regex::new(&re_pattern)
                && re.is_match(&lower)
            {
                return Err(SshError::ReadOnlyViolation {
                    command: command.to_string(),
                });
            }
        }

        // 2. Disallow pipeline redirection to files or write commands
        if lower.contains("redirect") || lower.contains(">") {
            return Err(SshError::ReadOnlyViolation {
                command: command.to_string(),
            });
        }

        // 3. Must match an allowed read-only show prefix
        let is_allowed = Self::ALLOWED_SHOW_PREFIXES
            .iter()
            .any(|&prefix| lower.starts_with(prefix));

        if is_allowed {
            Ok(())
        } else {
            Err(SshError::ReadOnlyViolation {
                command: command.to_string(),
            })
        }
    }
}

/// Abstract SSH transport trait for communicating with Cisco switches.
pub trait SshTransport: Send + Sync {
    /// Execute a read-only command on the target switch.
    fn execute(&self, target: &DeviceTarget, command: &str) -> Result<CommandOutput, SshError>;

    /// Retrieve the presented host key from the target switch for verification.
    fn get_host_key(&self, target: &DeviceTarget) -> Result<(String, Vec<u8>), SshError>;
}
