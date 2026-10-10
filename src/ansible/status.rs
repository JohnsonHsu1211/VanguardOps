use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeviceExecutionState {
    Pending,
    Running,
    Ok,
    Changed,
    Failed,
    Unreachable,
}

impl DeviceExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceExecutionState::Pending => "PENDING",
            DeviceExecutionState::Running => "RUNNING",
            DeviceExecutionState::Ok => "OK",
            DeviceExecutionState::Changed => "CHANGED",
            DeviceExecutionState::Failed => "FAILED",
            DeviceExecutionState::Unreachable => "UNREACHABLE",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self,
            DeviceExecutionState::Ok | DeviceExecutionState::Changed
        )
    }
}

/// Structured per-device outcome parsed from Ansible PLAY RECAP output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeviceJobStatus {
    pub device: String,
    pub state: DeviceExecutionState,
    pub ok_count: u32,
    pub changed_count: u32,
    pub failed_count: u32,
    pub unreachable_count: u32,
    pub skipped_count: u32,
    pub details: String,
}

impl DeviceJobStatus {
    pub fn new(device: impl Into<String>) -> Self {
        Self {
            device: device.into(),
            state: DeviceExecutionState::Pending,
            ok_count: 0,
            changed_count: 0,
            failed_count: 0,
            unreachable_count: 0,
            skipped_count: 0,
            details: String::new(),
        }
    }
}

/// Overall lifecycle state of an Ansible automation job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobState {
    Pending,
    AwaitingApproval,
    Approved,
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

impl JobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobState::Pending => "PENDING",
            JobState::AwaitingApproval => "AWAITING_APPROVAL",
            JobState::Approved => "APPROVED",
            JobState::Running => "RUNNING",
            JobState::Completed => "COMPLETED",
            JobState::Failed => "FAILED",
            JobState::TimedOut => "TIMED_OUT",
            JobState::Cancelled => "CANCELLED",
        }
    }
}

/// Parser extracting per-device execution status from stdout.
pub struct StatusParser;

impl StatusParser {
    /// Parse Ansible PLAY RECAP section into per-device structured status records.
    pub fn parse_play_recap(stdout: &str) -> HashMap<String, DeviceJobStatus> {
        let mut results = HashMap::new();
        let mut in_recap = false;

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("PLAY RECAP") {
                in_recap = true;
                continue;
            }

            if in_recap {
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(colon_idx) = trimmed.find(':') {
                    let host = trimmed[..colon_idx].trim().to_string();
                    let stats_part = &trimmed[colon_idx + 1..];

                    let mut ok = 0u32;
                    let mut changed = 0u32;
                    let mut unreachable = 0u32;
                    let mut failed = 0u32;
                    let mut skipped = 0u32;

                    for kv in stats_part.split_whitespace() {
                        let mut pair = kv.split('=');
                        if let (Some(k), Some(v)) = (pair.next(), pair.next()) {
                            let count = v.parse::<u32>().unwrap_or(0);
                            match k {
                                "ok" => ok = count,
                                "changed" => changed = count,
                                "unreachable" => unreachable = count,
                                "failed" => failed = count,
                                "skipped" => skipped = count,
                                _ => {}
                            }
                        }
                    }

                    let state = if unreachable > 0 {
                        DeviceExecutionState::Unreachable
                    } else if failed > 0 {
                        DeviceExecutionState::Failed
                    } else if changed > 0 {
                        DeviceExecutionState::Changed
                    } else if ok > 0 {
                        DeviceExecutionState::Ok
                    } else {
                        DeviceExecutionState::Pending
                    };

                    let details = format!(
                        "ok={ok} changed={changed} unreachable={unreachable} failed={failed} skipped={skipped}"
                    );

                    results.insert(
                        host.clone(),
                        DeviceJobStatus {
                            device: host,
                            state,
                            ok_count: ok,
                            changed_count: changed,
                            failed_count: failed,
                            unreachable_count: unreachable,
                            skipped_count: skipped,
                            details,
                        },
                    );
                }
            }
        }

        results
    }
}
