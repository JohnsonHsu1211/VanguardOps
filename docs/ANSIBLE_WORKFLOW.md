# Operator Workflow: Ansible Subprocess Automation Integration

## 1. Overview & Security Architecture
Milestone 3 provides a secure subprocess adapter for executing Ansible automation playbooks across the campus network switch infrastructure. To ensure complete operational safety in mission-critical IDF environments, the adapter implements:

- **Dedicated Subprocess Adapter**: Direct execution of `/usr/bin/ansible-playbook` using argument arrays without shell interpolation (`sh -c` or `bash -c`), eliminating command-injection risks.
- **Write Operations Disabled by Default**: All configuration-mutating write operations are locked out unless explicitly enabled in configuration.
- **Pre-Flight Dry-Run Support**: Operators preview all proposed changes via `--check` and `--diff` prior to live execution.
- **Two-Tier Approval Gates**: Mandatory authorization sign-off by a designated reviewer before any mutating playbook runs.
- **Strict Playbook Path Whitelist**: Canonical path restrictions ensuring playbooks execute solely from approved, trusted directories without directory traversal (`..`).
- **Validated Target Inventory**: Validates target hostnames and IPs against the live inventory database and RFC 1123 constraints.
- **Per-Device Structured Status**: Real-time parsing of task outcomes and `PLAY RECAP` (`ok`, `changed`, `unreachable`, `failed`).
- **Automated Post-Change Verification**: Post-run health checks inspecting device reachability, interface error counts, and environmental stability.
- **Secret Redaction & Audit Trail**: Passwords, tokens, vault secrets, and private keys are scrubbed from logs and stored in an immutable audit trail.

---

## 2. Standard 7-Step Operator Workflow

```
[1. Target Selection] -> [2. Playbook Selection] -> [3. Dry-Run Check]
                                                            |
                                                            v
[6. Post-Verification] <- [5. Live Execution]   <- [4. Approval Gate]
         |
         v
[7. Audit Trail Review]
```

### Step 1: Target Selection & Validation
Operators select the scope of target switches to automate:
- **Single Switch**: Target an individual switch (e.g., `sw-idf1a-01.hq.net`).
- **Rack Scope**: Target all switches within a rack (e.g., `rack:rack-1a-01`).
- **IDF Closet Scope**: Target all switches in an IDF closet (e.g., `closet:closet-idf1a`).
- **Validation**: The system validates that each target exists in the inventory, has a valid RFC 1123 hostname syntax, contains no shell metacharacters, and is not offline.

### Step 2: Playbook Selection & Path Verification
Operators select a playbook from the approved whitelist catalog:
- **Read-Only / Diagnostic Playbooks**:
  - `backup_cisco_config.yml`: Backup switch running configurations.
  - `verify_interfaces.yml`: Validate port operational states and VLANs.
  - `collect_diagnostics.yml`: Capture CPU, memory, PSU, and PoE telemetry.
- **Write / Configuration Playbooks**:
  - `update_access_vlan.yml`: Reconfigure access VLANs on specified ports.
  - `remediate_errdisabled_port.yml`: Bounce err-disabled ports after violation resolution.
  - `sync_ntp_syslog.yml`: Synchronize NTP and Syslog collectors across campus switches.
- **Security Constraint**: Paths must resolve strictly within the configured `trusted_root` (e.g., `/etc/idf/playbooks`). Directory traversal (`..`), symbolic link escapes, and unapproved filenames are rejected immediately.


### Step 3: Pre-Flight Dry-Run (`--check --diff`)
Prior to any configuration change, the operator initiates a dry-run:
```bash
ansible-playbook -i /etc/idf/inventory/hosts.ini \
  --limit sw-idf1a-01.hq.net \
  --check --diff \
  -v /etc/idf/playbooks/update_access_vlan.yml
```
- **Safety**: Dry-run mode passes `--check` and `--diff`, allowing Ansible to simulate tasks and produce diffs without making actual configuration changes on the switch.
- **Review**: The operator reviews the proposed diff and per-device status.

### Step 4: Explicit Change Approval Gate
For write/mutating playbooks:
1. **Master Lock Verification**: Verifies `write_operations_enabled == true` in system configuration. (Default is `false` - write operations disabled).
2. **Approval Request**: The operator submits a request containing:
   - Target devices and proposed playbook.
   - Justification and change ticket reference (e.g., `CHG-89412`).
   - Dry-run verification results.
3. **Peer Review**: A designated peer reviewer evaluates and approves the request:
   - Evaluates risk, scope, and timing window.
   - Signs off with reviewer identity, timestamp, and authorization notes.
   - Rejections abort execution and log the decision to the audit trail.

### Step 5: Bounded Subprocess Execution & Monitoring
Once authorized, the subprocess adapter spawns the runner:
- Arguments are passed as an explicit array without invoking a shell.
- Execution timeout is enforced (default: 60s, configurable per job).
- The operator monitors per-device status in real-time as tasks execute:
  - `PENDING` -> `RUNNING` -> `OK` / `CHANGED` / `FAILED` / `UNREACHABLE`.

### Step 6: Automated Post-Change Health Verification
Immediately upon playbook completion, the adapter runs automated verification:
1. **Reachability Check**: Confirms management IP responsiveness.
2. **Interface Health Check**: Verifies that no ports transitioned to `err-disabled` and CRC error rates did not surge.
3. **Chassis & Thermal Check**: Checks internal temperatures (<= 50°C), PSU redundancy, and fan states.
- The outcome (`PASSED`, `WARNING`, `FAILED`) is appended to the job record.

### Step 7: Audit Trail Review & Incident Traceability
Every job submission, approval decision, command string, per-device recap, and verification report is recorded in the immutable audit trail:
- All sensitive credentials and hashes are redacted (`[REDACTED]`).
- Records can be queried by `job_id`, target device hostname, or operator identity.

---

## 3. Operator Keyboard & TUI Controls
When running in the interactive TUI:
- Press `[6]` or `Tab` to navigate to the **Ansible & Audit** view.
- Press `[d]` to trigger a pre-flight Dry-Run on the selected switch.
- Press `[a]` to inspect and approve a staged change request.
- Press `[r]` to refresh audit entries and per-device execution statuses.
- Use `[↑/↓]` to scroll through historical jobs and per-device recaps.
