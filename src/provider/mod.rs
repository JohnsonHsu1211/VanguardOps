pub mod cisco;
pub mod concurrency;
pub mod credentials;
pub mod sanitizer;
pub mod ssh;

use std::path::PathBuf;
use std::time::Duration;

pub use concurrency::{ConcurrencyError, ConcurrencyLimiter};
pub use credentials::{
    AuthMethod, CredentialError, CredentialRef, DeviceCredentials, SecretString,
};
pub use sanitizer::LogSanitizer;
pub use ssh::host_key::{HostKeyError, HostKeyFingerprint, HostKeyPolicy, KnownHostsEntry};
pub use ssh::{CommandOutput, ReadOnlyCommandValidator, SshError, SshTransport};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("SSH transport error for {target}: {source}")]
    Ssh {
        target: String,
        #[source]
        source: SshError,
    },

    #[error("Platform parse error for {target}: {source}")]
    Parse {
        target: String,
        #[source]
        source: cisco::parsers::ParseError,
    },

    #[error("Concurrency limit exceeded for {target}: {source}")]
    Concurrency {
        target: String,
        #[source]
        source: ConcurrencyError,
    },

    #[error("Operation on {target} timed out after {duration:?}")]
    Timeout { target: String, duration: Duration },

    #[error("Target {target} platform mismatch: expected {expected}, found {actual}")]
    PlatformMismatch {
        target: String,
        expected: String,
        actual: String,
    },
}

/// Target device network operating system platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevicePlatform {
    /// Cisco IOS-XE (e.g. 17.9.4a on Catalyst 9300)
    CiscoIosXe,
    /// Classic Cisco IOS (e.g. 15.x on Catalyst 2960/3750)
    CiscoIos,
    /// Generic / configurable Cisco profile
    GenericCisco,
}

impl DevicePlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            DevicePlatform::CiscoIosXe => "Cisco IOS-XE",
            DevicePlatform::CiscoIos => "Cisco IOS",
            DevicePlatform::GenericCisco => "Generic Cisco",
        }
    }
}

/// Timeout configurations for switch operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutConfig {
    pub connect_timeout: Duration,
    pub command_timeout: Duration,
    pub session_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(10),
            session_timeout: Duration::from_secs(15),
        }
    }
}

/// Target connection configuration for a remote switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTarget {
    pub device_id: String,
    pub hostname: String,
    pub ip: String,
    pub port: u16,
    pub platform: DevicePlatform,
    pub credentials: DeviceCredentials,
    pub host_key_policy: HostKeyPolicy,
    pub timeouts: TimeoutConfig,
    pub known_hosts_path: Option<PathBuf>,
}

impl DeviceTarget {
    pub fn new(
        device_id: impl Into<String>,
        hostname: impl Into<String>,
        ip: impl Into<String>,
        credentials: DeviceCredentials,
        host_key_policy: HostKeyPolicy,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            hostname: hostname.into(),
            ip: ip.into(),
            port: 22,
            platform: DevicePlatform::CiscoIosXe,
            credentials,
            host_key_policy,
            timeouts: TimeoutConfig::default(),
            known_hosts_path: None,
        }
    }
}
