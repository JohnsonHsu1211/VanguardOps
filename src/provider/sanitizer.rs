use regex::Regex;
use std::sync::LazyLock;

static PRIVATE_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z0-9 ]+ KEY-----.*?-----END [A-Z0-9 ]+ KEY-----")
        .expect("valid regex")
});

static CISCO_SECRET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(enable\s+secret|username\s+\S+\s+secret|secret|password)\s+(?:\d\s+)?([^\s,;]+)",
    )
    .expect("valid regex")
});

static SENSITIVE_KV_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(community|token|passphrase|auth-key)\s+([^\s,;]+)").expect("valid regex")
});

static CRYPT_HASH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$[0-9a-zA-Z]+\$[^\s,;]+").expect("valid regex"));

/// Log and string sanitizer to prevent credentials, secrets, and private keys
/// from being leaked into error messages or log files.
pub struct LogSanitizer;

impl LogSanitizer {
    /// Redact sensitive information from text.
    pub fn sanitize(input: &str) -> String {
        let step1 = PRIVATE_KEY_REGEX.replace_all(input, "[REDACTED PRIVATE KEY]");
        let step2 = CISCO_SECRET_REGEX.replace_all(&step1, "$1 [REDACTED]");
        let step3 = SENSITIVE_KV_REGEX.replace_all(&step2, "$1 [REDACTED]");
        let step4 = CRYPT_HASH_REGEX.replace_all(&step3, "[REDACTED HASH]");
        step4.into_owned()
    }

    /// Format a sanitized error string.
    pub fn sanitize_error(err: &impl std::fmt::Display) -> String {
        Self::sanitize(&err.to_string())
    }
}
