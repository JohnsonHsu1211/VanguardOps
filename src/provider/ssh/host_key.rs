use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostKeyError {
    #[error("SSH host key mismatch for {host}: expected {expected}, but server presented {actual}")]
    HostKeyMismatch {
        host: String,
        expected: String,
        actual: String,
    },
    #[error("Untrusted SSH host key for {host}: key {fingerprint} not found in trusted database")]
    HostKeyUntrusted { host: String, fingerprint: String },
    #[error("Invalid host key format: {0}")]
    InvalidKeyFormat(String),
}

/// Helper for computing standard OpenSSH host key fingerprints.
pub struct HostKeyFingerprint;

impl HostKeyFingerprint {
    /// Compute OpenSSH standard SHA256 base64 fingerprint: "SHA256:<base64-without-padding>"
    pub fn sha256(raw_key: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_key);
        let hash = hasher.finalize();
        let b64 = BASE64_STANDARD.encode(hash);
        let trimmed = b64.trim_end_matches('=');
        format!("SHA256:{trimmed}")
    }

    /// Compute hex fingerprint: "aa:bb:cc:..."
    pub fn hex(raw_key: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_key);
        let hash = hasher.finalize();
        hash.iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

/// An entry parsed from a known_hosts database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHostsEntry {
    pub host_pattern: String,
    pub key_type: String,
    pub key_bytes: Vec<u8>,
    pub fingerprint: String,
}

impl KnownHostsEntry {
    pub fn new(
        host_pattern: impl Into<String>,
        key_type: impl Into<String>,
        key_bytes: Vec<u8>,
    ) -> Self {
        let key_type = key_type.into();
        let fingerprint = HostKeyFingerprint::sha256(&key_bytes);
        Self {
            host_pattern: host_pattern.into(),
            key_type,
            key_bytes,
            fingerprint,
        }
    }

    pub fn matches_host(&self, host: &str, port: u16) -> bool {
        let candidates = [
            host.to_string(),
            format!("[{host}]:{port}"),
            format!("{host}:{port}"),
        ];

        for part in self.host_pattern.split(',') {
            let part = part.trim();
            if candidates.iter().any(|c| c == part) {
                return true;
            }
        }
        false
    }

    /// Parse a single OpenSSH known_hosts line.
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            return None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let host_pattern = parts[0];
        let key_type = parts[1];
        let key_b64 = parts[2];

        let key_bytes = BASE64_STANDARD.decode(key_b64).ok()?;
        Some(Self::new(host_pattern, key_type, key_bytes))
    }
}

/// Verification policy for remote SSH host keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyPolicy {
    /// Strict checking against known_hosts and/or explicitly pinned fingerprints.
    Strict {
        known_hosts: Vec<KnownHostsEntry>,
        pinned_fingerprints: HashMap<String, String>,
    },
    /// Direct pinned fingerprint for target device (e.g. "SHA256:...")
    Pinned(String),
    /// Accept first key seen, but strictly reject mismatches if already known.
    AcceptNew { known_hosts: Vec<KnownHostsEntry> },
}

impl HostKeyPolicy {
    pub fn strict_with_pinned(pinned: HashMap<String, String>) -> Self {
        Self::Strict {
            known_hosts: Vec::new(),
            pinned_fingerprints: pinned,
        }
    }

    pub fn pinned_fingerprint(fingerprint: impl Into<String>) -> Self {
        Self::Pinned(fingerprint.into())
    }
}

pub struct HostKeyVerifier;

impl HostKeyVerifier {
    pub fn verify(
        host: &str,
        port: u16,
        key_type: &str,
        raw_key: &[u8],
        policy: &HostKeyPolicy,
    ) -> Result<(), HostKeyError> {
        let presented_fp = HostKeyFingerprint::sha256(raw_key);

        match policy {
            HostKeyPolicy::Pinned(pinned_fp) => {
                if &presented_fp == pinned_fp {
                    Ok(())
                } else {
                    Err(HostKeyError::HostKeyMismatch {
                        host: host.to_string(),
                        expected: pinned_fp.clone(),
                        actual: presented_fp,
                    })
                }
            }
            HostKeyPolicy::Strict {
                known_hosts,
                pinned_fingerprints,
            } => {
                // 1. Check pinned fingerprints first
                if let Some(expected_fp) = pinned_fingerprints.get(host) {
                    if expected_fp == &presented_fp {
                        return Ok(());
                    } else {
                        return Err(HostKeyError::HostKeyMismatch {
                            host: host.to_string(),
                            expected: expected_fp.clone(),
                            actual: presented_fp,
                        });
                    }
                }

                // 2. Check known_hosts
                let matching_entries: Vec<&KnownHostsEntry> = known_hosts
                    .iter()
                    .filter(|e| e.matches_host(host, port) && e.key_type == key_type)
                    .collect();

                if matching_entries.is_empty() {
                    return Err(HostKeyError::HostKeyUntrusted {
                        host: host.to_string(),
                        fingerprint: presented_fp,
                    });
                }

                if matching_entries.iter().any(|e| e.key_bytes == raw_key) {
                    Ok(())
                } else {
                    Err(HostKeyError::HostKeyMismatch {
                        host: host.to_string(),
                        expected: matching_entries[0].fingerprint.clone(),
                        actual: presented_fp,
                    })
                }
            }
            HostKeyPolicy::AcceptNew { known_hosts } => {
                let matching_entries: Vec<&KnownHostsEntry> = known_hosts
                    .iter()
                    .filter(|e| e.matches_host(host, port) && e.key_type == key_type)
                    .collect();

                if matching_entries.is_empty() {
                    // New host, accept
                    Ok(())
                } else if matching_entries.iter().any(|e| e.key_bytes == raw_key) {
                    Ok(())
                } else {
                    Err(HostKeyError::HostKeyMismatch {
                        host: host.to_string(),
                        expected: matching_entries[0].fingerprint.clone(),
                        actual: presented_fp,
                    })
                }
            }
        }
    }
}
