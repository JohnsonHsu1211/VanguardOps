use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::provider::DeviceTarget;
use crate::provider::cisco::fixtures::*;
use crate::provider::ssh::host_key::{HostKeyFingerprint, HostKeyVerifier};
use crate::provider::ssh::{CommandOutput, ReadOnlyCommandValidator, SshError, SshTransport};

/// Deterministic Mock SSH Transport for Cisco Switch integration.
#[derive(Debug)]
pub struct MockSshTransport {
    command_responses: Mutex<HashMap<String, String>>,
    host_keys: Mutex<HashMap<String, (String, Vec<u8>)>>, // host -> (key_type, raw_key)
    simulated_failure: Mutex<Option<MockFailureMode>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockFailureMode {
    ConnectionRefused,
    AuthFailure,
    Timeout(Duration),
    CorruptedHostKey,
}

impl Default for MockSshTransport {
    fn default() -> Self {
        Self::new_cisco_c9300()
    }
}

impl MockSshTransport {
    /// Create a mock transport pre-loaded with Catalyst 9300 IOS-XE 17.9.4a responses.
    pub fn new_cisco_c9300() -> Self {
        let mut responses = HashMap::new();
        responses.insert(
            "show version".to_string(),
            SHOW_VERSION_C9300_48P.to_string(),
        );
        responses.insert(
            "show interfaces status".to_string(),
            SHOW_INTERFACES_STATUS_C9300_48P.to_string(),
        );
        responses.insert(
            "show interfaces".to_string(),
            SHOW_INTERFACES_DETAIL_C9300.to_string(),
        );
        responses.insert(
            "show environment all".to_string(),
            SHOW_ENVIRONMENT_ALL_C9300.to_string(),
        );
        responses.insert(
            "show power inline".to_string(),
            SHOW_POWER_INLINE_C9300.to_string(),
        );
        responses.insert(
            "show processes cpu sorted".to_string(),
            SHOW_PROCESSES_CPU_C9300.to_string(),
        );
        responses.insert(
            "show processes cpu".to_string(),
            SHOW_PROCESSES_CPU_C9300.to_string(),
        );
        responses.insert(
            "show processes memory".to_string(),
            SHOW_PROCESSES_MEMORY_C9300.to_string(),
        );
        responses.insert(
            "show switch".to_string(),
            SHOW_SWITCH_STACK_C9300.to_string(),
        );

        let mut host_keys = HashMap::new();
        let dummy_key_bytes = b"cisco-catalyst-9300-test-ed25519-key-bytes-0012".to_vec();
        host_keys.insert(
            "default".to_string(),
            ("ssh-ed25519".to_string(), dummy_key_bytes),
        );

        Self {
            command_responses: Mutex::new(responses),
            host_keys: Mutex::new(host_keys),
            simulated_failure: Mutex::new(None),
        }
    }

    pub fn set_response(&self, command: impl Into<String>, response: impl Into<String>) {
        let mut map = self.command_responses.lock().unwrap();
        map.insert(command.into(), response.into());
    }

    pub fn set_failure(&self, failure: Option<MockFailureMode>) {
        let mut sim = self.simulated_failure.lock().unwrap();
        *sim = failure;
    }

    pub fn set_host_key(
        &self,
        host: impl Into<String>,
        key_type: impl Into<String>,
        key_bytes: Vec<u8>,
    ) {
        let mut map = self.host_keys.lock().unwrap();
        map.insert(host.into(), (key_type.into(), key_bytes));
    }

    pub fn get_fingerprint(&self, host: &str) -> String {
        let map = self.host_keys.lock().unwrap();
        let (_, bytes) = map.get(host).or_else(|| map.get("default")).unwrap();
        HostKeyFingerprint::sha256(bytes)
    }
}

impl SshTransport for MockSshTransport {
    fn execute(&self, target: &DeviceTarget, command: &str) -> Result<CommandOutput, SshError> {
        // 1. Mandatory Read-Only Check
        ReadOnlyCommandValidator::validate(command)?;

        // 2. Host Key Verification
        let (key_type, raw_key) = self.get_host_key(target)?;
        HostKeyVerifier::verify(
            &target.ip,
            target.port,
            &key_type,
            &raw_key,
            &target.host_key_policy,
        )?;

        // 3. Simulated failure mode check
        if let Some(ref fail) = *self.simulated_failure.lock().unwrap() {
            match fail {
                MockFailureMode::ConnectionRefused => {
                    return Err(SshError::ConnectionFailed {
                        host: target.ip.clone(),
                        port: target.port,
                        reason: "Connection refused (mock)".to_string(),
                    });
                }
                MockFailureMode::AuthFailure => {
                    return Err(SshError::AuthFailed {
                        username: target.credentials.username.clone(),
                        host: target.ip.clone(),
                        details: "Invalid password / rejected credentials (mock)".to_string(),
                    });
                }
                MockFailureMode::Timeout(d) => {
                    return Err(SshError::Timeout {
                        phase: "command execution".to_string(),
                        duration: *d,
                    });
                }
                MockFailureMode::CorruptedHostKey => {}
            }
        }

        // 4. Return matching response
        let map = self.command_responses.lock().unwrap();
        let clean_cmd = command.trim();
        if let Some(resp) = map.get(clean_cmd) {
            Ok(CommandOutput::success(command, resp.clone()))
        } else {
            Ok(CommandOutput::success(command, String::new()))
        }
    }

    fn get_host_key(&self, target: &DeviceTarget) -> Result<(String, Vec<u8>), SshError> {
        let fail = self.simulated_failure.lock().unwrap().clone();
        if let Some(MockFailureMode::CorruptedHostKey) = fail {
            return Ok(("ssh-ed25519".to_string(), b"corrupted-invalid-key".to_vec()));
        }

        let map = self.host_keys.lock().unwrap();
        if let Some((ktype, bytes)) = map
            .get(&target.ip)
            .or_else(|| map.get(&target.hostname))
            .or_else(|| map.get("default"))
        {
            Ok((ktype.clone(), bytes.clone()))
        } else {
            Err(SshError::ConnectionFailed {
                host: target.ip.clone(),
                port: target.port,
                reason: "Host key not found in mock store".to_string(),
            })
        }
    }
}
