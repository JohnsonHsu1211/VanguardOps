use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum SubprocessError {
    #[error("Failed to spawn process '{program}': {source}")]
    SpawnFailed {
        program: String,
        source: std::io::Error,
    },
    #[error("Process '{program}' execution failed: {source}")]
    ExecutionFailed {
        program: String,
        source: std::io::Error,
    },
    #[error("Subprocess timed out after {0:?}")]
    TimedOut(Duration),
}

/// Output captured from a subprocess execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubprocessOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub timed_out: bool,
}

impl SubprocessOutput {
    pub fn success(stdout: impl Into<String>, duration: Duration) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
            duration,
            timed_out: false,
        }
    }

    pub fn failure(
        code: i32,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            exit_code: code,
            stdout: stdout.into(),
            stderr: stderr.into(),
            duration,
            timed_out: false,
        }
    }
}

/// Trait abstracting subprocess execution to allow mock-backed tests and development.
pub trait SubprocessRunner: Send + Sync {
    fn run(
        &self,
        program: &Path,
        args: &[String],
        env: &HashMap<String, String>,
        timeout: Duration,
    ) -> Result<SubprocessOutput, SubprocessError>;
}

/// Real subprocess execution engine using `std::process::Command` with argument arrays.
pub struct ProcessSubprocessRunner;

impl SubprocessRunner for ProcessSubprocessRunner {
    fn run(
        &self,
        program: &Path,
        args: &[String],
        env: &HashMap<String, String>,
        timeout: Duration,
    ) -> Result<SubprocessOutput, SubprocessError> {
        let start = Instant::now();

        // 1. Direct argument array execution without shell wrapping
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.envs(env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| SubprocessError::SpawnFailed {
            program: program.display().to_string(),
            source: e,
        })?;

        // 2. Timeout polling loop
        let poll_interval = Duration::from_millis(50);

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let output =
                        child
                            .wait_with_output()
                            .map_err(|e| SubprocessError::ExecutionFailed {
                                program: program.display().to_string(),
                                source: e,
                            })?;
                    let duration = start.elapsed();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = status
                        .code()
                        .unwrap_or(if status.success() { 0 } else { 1 });

                    return Ok(SubprocessOutput {
                        exit_code,
                        stdout,
                        stderr,
                        duration,
                        timed_out: false,
                    });
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    return Err(SubprocessError::ExecutionFailed {
                        program: program.display().to_string(),
                        source: e,
                    });
                }
            }
        }

        let duration = start.elapsed();
        Ok(SubprocessOutput {
            exit_code: 124, // Standard timeout exit code
            stdout: String::new(),
            stderr: format!("Process timed out after {timeout:?}"),
            duration,
            timed_out: true,
        })
    }
}

/// Mock subprocess runner delivering realistic Ansible playbook results
/// without executing external binaries or touching physical network devices.
pub struct MockSubprocessRunner {
    pub simulated_timeout: Mutex<Option<Duration>>,
    pub simulated_exit_code: Mutex<Option<i32>>,
    pub custom_responses: Mutex<HashMap<String, String>>,
}

impl Default for MockSubprocessRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSubprocessRunner {
    pub fn new() -> Self {
        Self {
            simulated_timeout: Mutex::new(None),
            simulated_exit_code: Mutex::new(None),
            custom_responses: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_timeout(&self, timeout: Option<Duration>) {
        *self.simulated_timeout.lock().unwrap() = timeout;
    }

    pub fn set_exit_code(&self, code: Option<i32>) {
        *self.simulated_exit_code.lock().unwrap() = code;
    }

    pub fn set_response(&self, playbook: impl Into<String>, stdout: impl Into<String>) {
        self.custom_responses
            .lock()
            .unwrap()
            .insert(playbook.into(), stdout.into());
    }
}

impl SubprocessRunner for MockSubprocessRunner {
    fn run(
        &self,
        _program: &Path,
        args: &[String],
        _env: &HashMap<String, String>,
        _timeout: Duration,
    ) -> Result<SubprocessOutput, SubprocessError> {
        let duration = Duration::from_millis(120);

        // Check simulated timeout
        if let Some(t) = *self.simulated_timeout.lock().unwrap() {
            return Ok(SubprocessOutput {
                exit_code: 124,
                stdout: String::new(),
                stderr: format!("Process timed out after {t:?}"),
                duration: t,
                timed_out: true,
            });
        }

        // Check simulated exit code
        if let Some(code) = *self.simulated_exit_code.lock().unwrap() {
            return Ok(SubprocessOutput::failure(
                code,
                "PLAY [Mock Execution Failed]\nTASK [Mock Task] fatal: [sw-idf1a-01.hq.net]: FAILED! => {\"msg\": \"Simulated failure\"}\n\nPLAY RECAP *********************************************************************\nsw-idf1a-01.hq.net : ok=1 changed=0 unreachable=0 failed=1 skipped=0 rescued=0 ignored=0",
                "Ansible execution returned non-zero code",
                duration,
            ));
        }

        // Extract playbook name and targets from args
        let playbook_arg = args.last().cloned().unwrap_or_default();
        let is_check_mode = args.iter().any(|a| a == "--check");

        // Check custom responses
        if let Some(custom) = self.custom_responses.lock().unwrap().get(&playbook_arg) {
            return Ok(SubprocessOutput::success(custom.clone(), duration));
        }

        // Extract targets from --limit
        let mut target_hosts = vec!["sw-idf1a-01.hq.net".to_string()];
        if let Some(limit_idx) = args.iter().position(|a| a == "--limit")
            && let Some(limit_val) = args.get(limit_idx + 1)
        {
            target_hosts = limit_val.split(',').map(|s| s.trim().to_string()).collect();
        }

        // Generate realistic deterministic output
        let mode_desc = if is_check_mode {
            "[DRY-RUN / CHECK MODE]"
        } else {
            "[LIVE RUN]"
        };
        let mut out =
            "PLAY [IDF Switch Infrastructure Automation] **********************************\n\
             TASK [Gathering Facts] *********************************************************\n"
                .to_string();

        for host in &target_hosts {
            out.push_str(&format!("ok: [{host}]\n"));
        }

        out.push_str(&format!(
            "\nTASK [Execute Automation Workflow {mode_desc}] ************************\n"
        ));

        for host in &target_hosts {
            if is_check_mode {
                out.push_str(&format!(
                    "changed: [{host}] => (item=check_diff_verified)\n"
                ));
            } else {
                out.push_str(&format!("changed: [{host}] => (item=config_applied)\n"));
            }
        }

        out.push_str(
            "\nPLAY RECAP *********************************************************************\n",
        );
        for host in &target_hosts {
            out.push_str(&format!("{host:<28} : ok=2    changed=1    unreachable=0    failed=0    skipped=0    rescued=0    ignored=0\n"));
        }

        Ok(SubprocessOutput::success(out, duration))
    }
}
