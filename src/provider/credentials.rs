use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A memory-scrubbed secret container whose Debug and Display outputs are strictly redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString {
    bytes: Vec<u8>,
}

impl SecretString {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            bytes: secret.into().into_bytes(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    /// Access the underlying secret bytes explicitly.
    pub fn expose_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Access the underlying secret as a string slice if valid UTF-8.
    pub fn expose_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        // Volatile wipe of memory
        for byte in self.bytes.iter_mut() {
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Environment variable '{0}' is not set")]
    EnvNotSet(String),
    #[error("Failed to read credential file '{path}': {source}")]
    FileReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Credential reference is empty or invalid")]
    InvalidReference,
}

/// Secure reference to a credential, resolving at connection time.
#[derive(Clone, PartialEq, Eq)]
pub enum CredentialRef {
    /// Read from environment variable (e.g. "CISCO_SSH_PASSWORD")
    Env(String),
    /// Read from a protected file path
    File(PathBuf),
    /// Direct in-memory secret string
    Secret(SecretString),
}

impl CredentialRef {
    pub fn from_env(var_name: impl Into<String>) -> Self {
        Self::Env(var_name.into())
    }

    pub fn from_file(path: impl AsRef<Path>) -> Self {
        Self::File(path.as_ref().to_path_buf())
    }

    pub fn from_secret(secret: impl Into<String>) -> Self {
        Self::Secret(SecretString::new(secret))
    }

    /// Resolves the credential reference to a SecretString.
    pub fn resolve(&self) -> Result<SecretString, CredentialError> {
        match self {
            CredentialRef::Env(var_name) => {
                let val = std::env::var(var_name)
                    .map_err(|_| CredentialError::EnvNotSet(var_name.clone()))?;
                Ok(SecretString::new(val))
            }
            CredentialRef::File(path) => {
                let content = fs::read(path).map_err(|e| CredentialError::FileReadFailed {
                    path: path.clone(),
                    source: e,
                })?;
                Ok(SecretString::from_bytes(&content))
            }
            CredentialRef::Secret(sec) => Ok(sec.clone()),
        }
    }
}

impl fmt::Debug for CredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialRef::Env(var) => write!(f, "CredentialRef::Env(\"{var}\")"),
            CredentialRef::File(p) => write!(f, "CredentialRef::File({:?})", p),
            CredentialRef::Secret(_) => write!(f, "CredentialRef::Secret([REDACTED])"),
        }
    }
}

/// Authentication methods supported for Cisco switch connections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthMethod {
    Password(CredentialRef),
    PrivateKey {
        key_ref: CredentialRef,
        passphrase: Option<CredentialRef>,
    },
    PasswordAndKey {
        password_ref: CredentialRef,
        key_ref: CredentialRef,
    },
}

/// Complete device authentication configuration with sanitized Debug printing.
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceCredentials {
    pub username: String,
    pub auth: AuthMethod,
}

impl DeviceCredentials {
    pub fn with_password(username: impl Into<String>, password_ref: CredentialRef) -> Self {
        Self {
            username: username.into(),
            auth: AuthMethod::Password(password_ref),
        }
    }

    pub fn with_key(
        username: impl Into<String>,
        key_ref: CredentialRef,
        passphrase: Option<CredentialRef>,
    ) -> Self {
        Self {
            username: username.into(),
            auth: AuthMethod::PrivateKey {
                key_ref,
                passphrase,
            },
        }
    }
}

impl fmt::Debug for DeviceCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceCredentials")
            .field("username", &self.username)
            .field("auth", &self.auth)
            .finish()
    }
}
