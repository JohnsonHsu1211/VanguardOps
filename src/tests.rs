use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    app::{App, DeviceStatusFilter, InterfaceFilter, InterfaceSortKey, Tab},
    mock::generate_mock_infrastructure,
    model::*,
    ui::render_app,
};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_mock_infrastructure_integrity() {
    let net = generate_mock_infrastructure();

    assert_eq!(net.total_closets(), 5);
    assert_eq!(net.total_switches(), 10);
    assert!(net.total_interfaces() > 200);
    assert!(net.active_interfaces() > 50);
    assert!(net.err_interfaces() >= 1);
    assert!(net.total_poe_draw() > 0.0);
    assert!(net.total_poe_budget() > 0.0);
    assert!(!net.alerts.is_empty());

    // Check specific known switches exist
    assert!(net.find_switch("sw-mdf-core01").is_some());
    assert!(net.find_switch("sw-idf1a-01").is_some());
    assert!(net.find_switch("sw-idf2b-02").is_some());
    assert!(net.find_switch("sw-idfwh-01").is_some());

    // Check BPDU-guard err-disabled port on idf1a-01
    let sw1a = net.find_switch("sw-idf1a-01").unwrap();
    let port14 = sw1a
        .interfaces
        .iter()
        .find(|i| i.name == "Gi1/0/14")
        .unwrap();
    assert_eq!(port14.oper_status, OperStatus::ErrDisabled);
}

#[test]
fn test_app_initialization_and_tabs() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    assert_eq!(app.active_tab, Tab::Dashboard);
    assert!(!app.should_quit);

    // Number keys tab switching
    app.handle_key(make_key(KeyCode::Char('2')));
    assert_eq!(app.active_tab, Tab::Inventory);

    app.handle_key(make_key(KeyCode::Char('3')));
    assert_eq!(app.active_tab, Tab::SwitchDetail);

    app.handle_key(make_key(KeyCode::Char('4')));
    assert_eq!(app.active_tab, Tab::Interfaces);

    app.handle_key(make_key(KeyCode::Char('5')));
    assert_eq!(app.active_tab, Tab::Alerts);

    app.handle_key(make_key(KeyCode::Char('1')));
    assert_eq!(app.active_tab, Tab::Dashboard);

    // Tab cycling
    app.handle_key(make_key(KeyCode::Tab));
    assert_eq!(app.active_tab, Tab::Inventory);

    app.handle_key(make_key(KeyCode::BackTab));
    assert_eq!(app.active_tab, Tab::Dashboard);

    // Quit key
    app.handle_key(make_key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn test_inventory_tree_navigation_and_expansion() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);
    app.active_tab = Tab::Inventory;

    let initial_nodes = app.visible_tree_nodes();
    assert!(!initial_nodes.is_empty());

    // Navigate down
    let start_cursor = app.tree_cursor;
    app.handle_key(make_key(KeyCode::Down));
    assert_eq!(app.tree_cursor, start_cursor + 1);

    // Navigate up
    app.handle_key(make_key(KeyCode::Up));
    assert_eq!(app.tree_cursor, start_cursor);

    // Collapse first closet
    app.tree_cursor = 0;
    app.handle_key(make_key(KeyCode::Left));
    assert!(!app.expanded_closets[0]);

    // Nodes count should decrease
    let collapsed_nodes = app.visible_tree_nodes();
    assert!(collapsed_nodes.len() < initial_nodes.len());

    // Expand again
    app.handle_key(make_key(KeyCode::Right));
    assert!(app.expanded_closets[0]);
    assert_eq!(app.visible_tree_nodes().len(), initial_nodes.len());

    // Filter toggle
    assert_eq!(app.status_filter, DeviceStatusFilter::All);
    app.handle_key(make_key(KeyCode::Char('f')));
    assert_eq!(app.status_filter, DeviceStatusFilter::OnlineOnly);
    app.handle_key(make_key(KeyCode::Char('f')));
    assert_eq!(app.status_filter, DeviceStatusFilter::IssuesOnly);
    let issue_nodes = app.visible_tree_nodes();
    assert!(issue_nodes.len() < initial_nodes.len());
}

#[test]
fn test_interfaces_sorting_and_filtering() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);
    app.active_tab = Tab::Interfaces;

    let initial_ports = app.filtered_interfaces();
    assert!(!initial_ports.is_empty());

    // Test sort cycling
    assert_eq!(app.interface_sort, InterfaceSortKey::PortName);
    app.handle_key(make_key(KeyCode::Char('s')));
    assert_eq!(app.interface_sort, InterfaceSortKey::Status);
    app.handle_key(make_key(KeyCode::Char('S')));
    assert!(!app.interface_sort_asc);

    // Test filter cycling
    assert_eq!(app.interface_filter, InterfaceFilter::All);
    app.handle_key(make_key(KeyCode::Char('f')));
    assert_eq!(app.interface_filter, InterfaceFilter::UpOnly);
    let up_ports = app.filtered_interfaces();
    for p in &up_ports {
        assert_eq!(p.oper_status, OperStatus::Up);
    }

    app.handle_key(make_key(KeyCode::Char('f')));
    assert_eq!(app.interface_filter, InterfaceFilter::DownOnly);
    let down_ports = app.filtered_interfaces();
    for p in &down_ports {
        assert_eq!(p.oper_status, OperStatus::Down);
    }

    app.handle_key(make_key(KeyCode::Char('f')));
    assert_eq!(app.interface_filter, InterfaceFilter::ErrDisabledOnly);
}

#[test]
fn test_search_and_jump() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    // Open search
    app.handle_key(make_key(KeyCode::Char('/')));
    assert!(app.search_active);

    // Type query "BPDU"
    for c in "bpdu".chars() {
        app.handle_key(make_key(KeyCode::Char(c)));
    }
    assert_eq!(app.search_query, "bpdu");
    assert!(!app.search_results.is_empty());

    // Press enter on the result to jump
    app.handle_key(make_key(KeyCode::Enter));
    assert!(!app.search_active);
    assert_eq!(app.active_tab, Tab::Interfaces);
    let current_sw = app.current_switch().unwrap();
    assert_eq!(current_sw.hostname, "sw-idf1a-01.hq.net");
}

#[test]
fn test_error_handling_and_recovery() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    assert!(app.error_modal.is_none());

    // Trigger simulated error
    app.handle_key(make_key(KeyCode::Char('e')));
    assert!(app.error_modal.is_some());

    // Dismiss error with Esc
    app.handle_key(make_key(KeyCode::Esc));
    assert!(app.error_modal.is_none());

    // Trigger again and dismiss with Enter
    app.handle_key(make_key(KeyCode::Char('e')));
    assert!(app.error_modal.is_some());
    app.handle_key(make_key(KeyCode::Enter));
    assert!(app.error_modal.is_none());
}

#[test]
fn test_help_and_port_modals() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    // Help modal
    app.handle_key(make_key(KeyCode::Char('?')));
    assert!(app.help_modal_open);
    app.handle_key(make_key(KeyCode::Esc));
    assert!(!app.help_modal_open);

    // Port detail modal from Interfaces tab
    app.active_tab = Tab::Interfaces;
    app.handle_key(make_key(KeyCode::Enter));
    assert!(app.port_detail_modal_open);
    app.handle_key(make_key(KeyCode::Esc));
    assert!(!app.port_detail_modal_open);
}

#[test]
fn test_ui_rendering_headless_all_tabs() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    let backend = TestBackend::new(120, 35);
    let mut terminal = Terminal::new(backend).unwrap();

    // Render each tab to ensure no panics
    for tab in Tab::ALL {
        app.active_tab = tab;
        terminal.draw(|f| render_app(f, &app)).unwrap();
    }

    // Render with Help modal open
    app.help_modal_open = true;
    terminal.draw(|f| render_app(f, &app)).unwrap();
    app.help_modal_open = false;

    // Render with Search modal open
    app.search_active = true;
    app.search_query = "cisco".to_string();
    terminal.draw(|f| render_app(f, &app)).unwrap();
    app.search_active = false;

    // Render with Error modal open
    app.error_modal = Some("Mock Test Error".to_string());
    terminal.draw(|f| render_app(f, &app)).unwrap();
    app.error_modal = None;

    // Render with Port Detail modal open
    app.port_detail_modal_open = true;
    terminal.draw(|f| render_app(f, &app)).unwrap();
    app.port_detail_modal_open = false;
}

#[test]
fn test_ui_rendering_compact_small_terminal() {
    let net = generate_mock_infrastructure();
    let app = App::new(net);

    // Small terminal (60x15), should render compact fallback without panicking
    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| render_app(f, &app)).unwrap();
}

// =========================================================================
// Milestone 2: Read-Only Cisco Switch Integration Tests
// =========================================================================

use crate::provider::cisco::CiscoSwitchProvider;
use crate::provider::cisco::fixtures::*;
use crate::provider::cisco::parsers::*;
use crate::provider::concurrency::ConcurrencyLimiter;
use crate::provider::credentials::*;
use crate::provider::sanitizer::LogSanitizer;
use crate::provider::ssh::host_key::*;
use crate::provider::ssh::mock::*;
use crate::provider::ssh::*;
use crate::provider::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_read_only_command_validator() {
    let valid_commands = [
        "show version",
        "show inventory",
        "show interfaces status",
        "show interfaces",
        "show environment all",
        "show power inline",
        "show processes cpu sorted",
        "show processes memory",
        "show switch",
        "show vlan brief",
        "show lldp neighbors detail",
        "show cdp neighbors detail",
        "show mac address-table",
        "ping 10.1.10.1",
    ];

    for cmd in valid_commands {
        assert!(
            ReadOnlyCommandValidator::validate(cmd).is_ok(),
            "Expected '{cmd}' to be accepted as read-only"
        );
    }

    let forbidden_commands = [
        "configure terminal",
        "conf t",
        "write memory",
        "write mem",
        "write erase",
        "copy running-config startup-config",
        "reload",
        "reboot",
        "shutdown",
        "interface Gi1/0/1",
        "no shutdown",
        "erase startup-config",
        "set system name foo",
        "clear ip route *",
        "debug ip packet",
        "crypto key generate rsa",
        "show version > bootflash:dump.txt",
        "",
    ];

    for cmd in forbidden_commands {
        match ReadOnlyCommandValidator::validate(cmd) {
            Err(SshError::ReadOnlyViolation { .. }) => {}
            other => panic!("Expected ReadOnlyViolation for '{cmd}', got {:?}", other),
        }
    }
}

#[test]
fn test_ssh_host_key_verification() {
    let dummy_key = b"cisco-ed25519-valid-public-key-bytes";
    let computed_fp = HostKeyFingerprint::sha256(dummy_key);
    assert!(computed_fp.starts_with("SHA256:"));

    let pinned_policy = HostKeyPolicy::pinned_fingerprint(&computed_fp);
    assert!(
        HostKeyVerifier::verify("10.1.10.11", 22, "ssh-ed25519", dummy_key, &pinned_policy).is_ok()
    );

    let wrong_pinned_policy = HostKeyPolicy::pinned_fingerprint("SHA256:wrongfingerprint12345");
    match HostKeyVerifier::verify(
        "10.1.10.11",
        22,
        "ssh-ed25519",
        dummy_key,
        &wrong_pinned_policy,
    ) {
        Err(HostKeyError::HostKeyMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, "SHA256:wrongfingerprint12345");
            assert_eq!(actual, computed_fp);
        }
        other => panic!("Expected HostKeyMismatch, got {:?}", other),
    }

    let entry = KnownHostsEntry::new("10.1.10.11", "ssh-ed25519", dummy_key.to_vec());
    let mut pinned_map = HashMap::new();
    pinned_map.insert("10.1.10.11".to_string(), computed_fp.clone());
    let strict_policy = HostKeyPolicy::Strict {
        known_hosts: vec![entry],
        pinned_fingerprints: pinned_map,
    };
    assert!(
        HostKeyVerifier::verify("10.1.10.11", 22, "ssh-ed25519", dummy_key, &strict_policy).is_ok()
    );

    let empty_strict_policy = HostKeyPolicy::Strict {
        known_hosts: Vec::new(),
        pinned_fingerprints: HashMap::new(),
    };
    match HostKeyVerifier::verify(
        "10.1.10.99",
        22,
        "ssh-ed25519",
        dummy_key,
        &empty_strict_policy,
    ) {
        Err(HostKeyError::HostKeyUntrusted { host, .. }) => {
            assert_eq!(host, "10.1.10.99");
        }
        other => panic!("Expected HostKeyUntrusted, got {:?}", other),
    }

    let accept_new_policy = HostKeyPolicy::AcceptNew {
        known_hosts: Vec::new(),
    };
    assert!(
        HostKeyVerifier::verify(
            "10.1.10.11",
            22,
            "ssh-ed25519",
            dummy_key,
            &accept_new_policy
        )
        .is_ok()
    );
}

#[test]
fn test_secure_credentials_and_redaction() {
    let secret = SecretString::new("SuperSecretPassword99");
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(secret.expose_str().unwrap(), "SuperSecretPassword99");

    unsafe {
        std::env::set_var("TEST_CISCO_PASS", "EnvPassword456");
    }
    let env_ref = CredentialRef::from_env("TEST_CISCO_PASS");
    let resolved = env_ref.resolve().expect("resolve env credential");
    assert_eq!(resolved.expose_str().unwrap(), "EnvPassword456");

    let missing_env = CredentialRef::from_env("NON_EXISTENT_VAR_12345");
    assert!(missing_env.resolve().is_err());

    let sec_ref = CredentialRef::from_secret("InMemPass789");
    assert_eq!(
        sec_ref.resolve().unwrap().expose_str().unwrap(),
        "InMemPass789"
    );
    assert_eq!(format!("{sec_ref:?}"), "CredentialRef::Secret([REDACTED])");

    let creds = DeviceCredentials::with_password("admin", sec_ref);
    let debug_out = format!("{creds:?}");
    assert!(debug_out.contains("admin"));
    assert!(!debug_out.contains("InMemPass789"));
    assert!(debug_out.contains("[REDACTED]"));
}

#[test]
fn test_log_sanitizer() {
    let sensitive_log = "Error connecting to 10.1.10.11: password CiscoSuperSecret123 was rejected; enable secret 5 $1$mERr$samplehash";
    let sanitized = LogSanitizer::sanitize(sensitive_log);

    assert!(!sanitized.contains("CiscoSuperSecret123"));
    assert!(!sanitized.contains("$1$mERr$samplehash"));
    assert!(sanitized.contains("password [REDACTED]"));
    assert!(sanitized.contains("secret [REDACTED]"));

    let key_log = "Presented key: -----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAA\n-----END OPENSSH PRIVATE KEY----- on port 22";
    let sanitized_key = LogSanitizer::sanitize(key_log);
    assert!(!sanitized_key.contains("b3BlbnNzaC"));
    assert!(sanitized_key.contains("[REDACTED PRIVATE KEY]"));
}

#[test]
fn test_concurrency_limiter() {
    let limiter = ConcurrencyLimiter::new(2);
    assert_eq!(limiter.max_permits(), 2);
    assert_eq!(limiter.active_permits(), 0);

    let permit1 = limiter
        .acquire(Duration::from_millis(50))
        .expect("acquire permit 1");
    assert_eq!(limiter.active_permits(), 1);

    let permit2 = limiter
        .acquire(Duration::from_millis(50))
        .expect("acquire permit 2");
    assert_eq!(limiter.active_permits(), 2);

    // 3rd permit must timeout since limit is 2
    let timeout_res = limiter.acquire(Duration::from_millis(20));
    assert!(timeout_res.is_err());

    // Dropping permit1 allows acquiring a new permit
    drop(permit1);
    assert_eq!(limiter.active_permits(), 1);

    let permit3 = limiter
        .acquire(Duration::from_millis(50))
        .expect("acquire permit 3");
    assert_eq!(limiter.active_permits(), 2);
    drop(permit2);
    drop(permit3);
    assert_eq!(limiter.active_permits(), 0);
}

#[test]
fn test_cisco_ios_xe_parsers() {
    // 1. show version 48P
    let sys48 = parse_show_version(SHOW_VERSION_C9300_48P).expect("parse show version 48P");
    assert_eq!(sys48.hostname, "sw-idf1a-01");
    assert_eq!(sys48.model, "C9300-48P");
    assert_eq!(sys48.serial, "FOC2419S3KL");
    assert_eq!(sys48.mac, "f4:0f:1b:2c:11:01");
    assert!(sys48.os_version.contains("17.09.04a"));
    assert!(sys48.uptime.contains("45 days"));

    // 2. show version 24P
    let sys24 = parse_show_version(SHOW_VERSION_C9300_24P).expect("parse show version 24P");
    assert_eq!(sys24.hostname, "sw-idf3c-01");
    assert_eq!(sys24.model, "C9300-24P");
    assert_eq!(sys24.serial, "FOC2510A88B");

    // 3. show interfaces status
    let statuses =
        parse_show_interfaces_status(SHOW_INTERFACES_STATUS_C9300_48P).expect("parse int status");
    assert!(statuses.len() >= 7);
    let port1 = statuses.iter().find(|i| i.port == "Gi1/0/1").unwrap();
    assert_eq!(port1.status, "connected");
    assert_eq!(port1.vlan, "10");

    let port14 = statuses.iter().find(|i| i.port == "Gi1/0/14").unwrap();
    assert_eq!(port14.status, "err-disabled");

    // 4. show interfaces detail
    let details =
        parse_show_interfaces_detail(SHOW_INTERFACES_DETAIL_C9300).expect("parse int detail");
    let gi1_detail = details.get("GigabitEthernet1/0/1").expect("find Gi1/0/1");
    assert_eq!(gi1_detail.mtu, 1500);
    assert!(gi1_detail.admin_up);
    assert!(gi1_detail.oper_up);
    assert_eq!(gi1_detail.rx_bytes, 14500000000);
    assert_eq!(gi1_detail.tx_bytes, 8300000000);
    assert_eq!(gi1_detail.errors_rx, 0);

    // 5. show environment all
    let env = parse_show_environment(SHOW_ENVIRONMENT_ALL_C9300).expect("parse environment");
    assert_eq!(env.temp_celsius, 48);
    assert_eq!(env.psu_1, PowerSupplyStatus::Ok);
    assert_eq!(env.psu_2, PowerSupplyStatus::RedundantOk);
    assert_eq!(env.fan_status, FanStatus::Normal);

    // 6. show power inline
    let poe = parse_show_power_inline(SHOW_POWER_INLINE_C9300).expect("parse power inline");
    assert_eq!(poe.budget_watts, 740.0);
    assert_eq!(poe.used_watts, 142.5);
    assert_eq!(*poe.port_power.get("Gi1/0/4").unwrap(), 7.4);
    assert_eq!(*poe.port_power.get("Gi1/0/6").unwrap(), 25.5);

    // 7. CPU and Memory
    let cpu = parse_show_processes_cpu(SHOW_PROCESSES_CPU_C9300).expect("parse cpu");
    assert_eq!(cpu, 22);

    let mem = parse_show_processes_memory(SHOW_PROCESSES_MEMORY_C9300).expect("parse mem");
    assert_eq!(mem, 57);

    // 8. Stack switch
    let stack = parse_show_switch(SHOW_SWITCH_STACK_C9300).expect("parse stack");
    assert_eq!(stack.len(), 2);
    assert_eq!(stack[0].role, StackRole::Master);
    assert_eq!(stack[1].role, StackRole::Standby);
}

#[test]
fn test_cisco_provider_full_mock_telemetry_sync() {
    let mock_transport = Arc::new(MockSshTransport::new_cisco_c9300());
    let limiter = ConcurrencyLimiter::new(4);
    let provider = CiscoSwitchProvider::new(mock_transport, limiter);

    let creds =
        DeviceCredentials::with_password("cisco_admin", CredentialRef::from_secret("CiscoSecret!"));
    let policy = HostKeyPolicy::AcceptNew {
        known_hosts: Vec::new(),
    };
    let target = DeviceTarget::new("sw-01", "sw-idf1a-01.hq.net", "10.1.10.11", creds, policy);

    let snapshot = provider.fetch_telemetry(&target).expect("fetch telemetry");
    assert_eq!(snapshot.system_info.hostname, "sw-idf1a-01");
    assert_eq!(snapshot.system_info.model, "C9300-48P");
    assert_eq!(snapshot.system_info.serial, "FOC2419S3KL");
    assert_eq!(snapshot.cpu_usage, 22);
    assert_eq!(snapshot.mem_usage, 57);
    assert_eq!(snapshot.environment.temp_celsius, 48);
    assert_eq!(snapshot.poe.budget_watts, 740.0);
    assert!(!snapshot.interfaces.is_empty());

    let net = generate_mock_infrastructure();
    let mut sw = net.find_switch("sw-idf1a-01").unwrap().clone();
    provider.sync_switch(&target, &mut sw).expect("sync switch");

    assert_eq!(sw.hostname, "sw-idf1a-01");
    assert_eq!(sw.model, "C9300-48P");
    assert_eq!(sw.serial, "FOC2419S3KL");
    assert_eq!(sw.cpu_usage, 22);
    assert_eq!(sw.mem_usage, 57);
}

#[test]
fn test_cisco_provider_timeout_and_error_sanitization() {
    let mock_transport = Arc::new(MockSshTransport::new_cisco_c9300());
    mock_transport.set_failure(Some(MockFailureMode::Timeout(Duration::from_secs(5))));

    let limiter = ConcurrencyLimiter::new(2);
    let provider = CiscoSwitchProvider::new(mock_transport.clone(), limiter);

    let creds =
        DeviceCredentials::with_password("admin", CredentialRef::from_secret("SuperSecretPass123"));
    let policy = HostKeyPolicy::AcceptNew {
        known_hosts: Vec::new(),
    };
    let target = DeviceTarget::new("sw-01", "sw-idf1a-01", "10.1.10.11", creds, policy);

    let res = provider.fetch_telemetry(&target);
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("timed out"));

    // Test auth failure sanitization
    mock_transport.set_failure(Some(MockFailureMode::AuthFailure));
    let auth_res = provider.fetch_telemetry(&target);
    assert!(auth_res.is_err());
    let sanitized_err = LogSanitizer::sanitize_error(&auth_res.unwrap_err());
    assert!(!sanitized_err.contains("SuperSecretPass123"));
}

#[test]
fn test_app_sync_cisco_switch() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    // Target switch is Cisco Catalyst 9300 in IDF-1A (closet 1, rack 0, switch 0)
    app.selected_switch = (1, 0, 0);
    app.active_tab = Tab::SwitchDetail;

    app.sync_selected_switch();
    assert!(app.notification.is_some());
    let notif = app.notification.clone().unwrap();
    assert_eq!(notif.level, crate::app::NotificationLevel::Success);
    assert!(
        notif
            .message
            .contains("Successfully synced read-only telemetry")
    );

    // Switch to a non-Cisco switch (e.g. Arista in closet 0, rack 0, switch 0)
    app.selected_switch = (0, 0, 0);
    app.sync_selected_switch();
    assert!(app.notification.is_some());
    let non_cisco_notif = app.notification.clone().unwrap();
    assert_eq!(
        non_cisco_notif.level,
        crate::app::NotificationLevel::Warning
    );
    assert!(non_cisco_notif.message.contains("is not Cisco IOS-XE"));
}

// =========================================================================
// Milestone 3: Ansible Subprocess Adapter & Automation Integration Tests
// =========================================================================

use crate::ansible::approval::*;
use crate::ansible::cmd::*;
use crate::ansible::playbook::*;
use crate::ansible::status::*;
use crate::ansible::target::*;
use crate::ansible::verification::*;
use crate::ansible::*;
use std::path::Path;

#[test]
fn test_ansible_target_validation() {
    let net = generate_mock_infrastructure();

    // 1. Valid single target
    let sel_single = TargetSelection::Single("sw-idf1a-01.hq.net".to_string());
    let val_single = TargetValidator::validate(&sel_single, &net, false).expect("validate single");
    assert_eq!(val_single.count(), 1);
    assert_eq!(val_single.limit_arg, "sw-idf1a-01.hq.net");

    // 2. Valid rack target (rack-1a-01 has 2 switches)
    let sel_rack = TargetSelection::Rack("rack-1a-01".to_string());
    let val_rack = TargetValidator::validate(&sel_rack, &net, false).expect("validate rack");
    assert_eq!(val_rack.count(), 2);
    assert!(val_rack.limit_arg.contains("sw-idf1a-01.hq.net"));
    assert!(val_rack.limit_arg.contains("sw-idf1a-02.hq.net"));

    // 3. Valid closet target (closet-idf1a has 3 switches across 2 racks)
    let sel_closet = TargetSelection::Closet("closet-idf1a".to_string());
    let val_closet = TargetValidator::validate(&sel_closet, &net, false).expect("validate closet");
    assert_eq!(val_closet.count(), 3);

    // 4. Rejection of invalid characters (shell injection hazards)
    let dangerous = [
        "sw-01; rm -rf /",
        "sw-01 & cat /etc/passwd",
        "sw-01 | sh",
        "sw-01`id`",
        "sw-01$(whoami)",
        "sw-01 > /tmp/out",
        "sw-01\nsw-02",
        "sw 01",
    ];

    for bad in dangerous {
        assert!(
            TargetValidator::validate_syntax(bad).is_err(),
            "Expected '{bad}' to fail syntax validation"
        );
    }

    // 5. Rejection of non-existent target
    let sel_unknown = TargetSelection::Single("nonexistent-switch.hq.net".to_string());
    match TargetValidator::validate(&sel_unknown, &net, false) {
        Err(TargetValidationError::DeviceNotFound { target }) => {
            assert_eq!(target, "nonexistent-switch.hq.net");
        }
        other => panic!("Expected DeviceNotFound, got {:?}", other),
    }

    // 6. Rejection of empty target list
    let sel_empty = TargetSelection::Multiple(Vec::new());
    assert_eq!(
        TargetValidator::validate(&sel_empty, &net, false),
        Err(TargetValidationError::EmptyTargets)
    );
}

#[test]
fn test_trusted_playbook_path_restrictions() {
    let reg = PlaybookRegistry::with_default_catalog("/etc/idf/playbooks");

    // 1. Valid approved playbook path resolution
    let path = reg
        .validate_path("backup_cisco_config.yml")
        .expect("valid playbook");
    assert_eq!(
        path,
        Path::new("/etc/idf/playbooks/backup_cisco_config.yml")
    );

    // 2. Directory traversal attempt rejection
    let traversal = reg.validate_path("../../../etc/shadow.yml");
    match traversal {
        Err(PlaybookSecurityError::DirectoryTraversal { .. }) => {}
        other => panic!("Expected DirectoryTraversal, got {:?}", other),
    }

    // 3. Invalid extension rejection (.sh instead of .yml/.yaml)
    let invalid_ext = reg.validate_path("backup_cisco_config.sh");
    match invalid_ext {
        Err(PlaybookSecurityError::InvalidExtension { .. }) => {}
        other => panic!("Expected InvalidExtension, got {:?}", other),
    }

    // 4. Unapproved playbook rejection
    let unapproved = reg.validate_path("unapproved_arbitrary_task.yml");
    match unapproved {
        Err(PlaybookSecurityError::NotApproved { name }) => {
            assert_eq!(name, "unapproved_arbitrary_task.yml");
        }
        other => panic!("Expected NotApproved, got {:?}", other),
    }

    // 5. Playbook classification (ReadOnly vs Write)
    let ro_pb = reg.find_playbook("backup_cisco_config.yml").unwrap();
    assert_eq!(ro_pb.playbook_type, PlaybookType::ReadOnly);
    assert!(!ro_pb.playbook_type.is_write());

    let write_pb = reg.find_playbook("update_access_vlan.yml").unwrap();
    assert_eq!(write_pb.playbook_type, PlaybookType::Write);
    assert!(write_pb.playbook_type.is_write());
}

#[test]
fn test_ansible_command_builder_array_execution() {
    let builder = AnsibleCommandBuilder::new(
        "/etc/idf/playbooks/backup_cisco_config.yml",
        "/etc/idf/inventory/hosts.ini",
    )
    .with_limit("sw-idf1a-01.hq.net")
    .with_check_mode(true)
    .with_diff_mode(true)
    .with_extra_var("vlan_id", "20")
    .with_extra_var("password", "SecretPassword999");

    let args = builder.build_args();

    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"/etc/idf/inventory/hosts.ini".to_string()));
    assert!(args.contains(&"--limit".to_string()));
    assert!(args.contains(&"sw-idf1a-01.hq.net".to_string()));
    assert!(args.contains(&"--check".to_string()));
    assert!(args.contains(&"--diff".to_string()));
    assert_eq!(
        args.last().unwrap(),
        "/etc/idf/playbooks/backup_cisco_config.yml"
    );

    let sanitized = builder.build_sanitized_display();
    assert!(!sanitized.contains("SecretPassword999"));
    assert!(sanitized.contains("[REDACTED]"));
}

#[test]
fn test_dry_run_execution() {
    let net = generate_mock_infrastructure();
    let adapter = AnsibleAdapter::new_mock("/etc/idf/playbooks");

    let req = JobRequest::dry_run(
        "verify_interfaces.yml",
        TargetSelection::Single("sw-idf1a-01.hq.net".to_string()),
        "operator_dan",
    );

    let result = adapter.submit_job(req, &net).expect("dry run submit");
    assert!(result.is_dry_run);
    assert_eq!(result.state, JobState::Completed);
    assert_eq!(result.exit_code, Some(0));
    assert!(result.sanitized_command.contains("--check"));
    assert!(result.sanitized_command.contains("--diff"));
    assert_eq!(
        result.verification.unwrap().status,
        VerificationStatus::Skipped
    );
}

#[test]
fn test_approval_gate_and_write_protection_disabled_by_default() {
    let net = generate_mock_infrastructure();
    let adapter = AnsibleAdapter::new_mock("/etc/idf/playbooks");

    // 1. By default, write operations must be locked
    assert!(!adapter.approval_gate.policy.write_operations_enabled);

    let write_req = JobRequest::live(
        "update_access_vlan.yml",
        TargetSelection::Single("sw-idf1a-01.hq.net".to_string()),
        "operator_bob",
        "VLAN change on access port",
    );

    // Live write submission must be rejected with WriteOperationsDisabled
    let err = adapter.submit_job(write_req.clone(), &net).unwrap_err();
    match err {
        AnsibleError::Approval(ApprovalError::WriteOperationsDisabled) => {}
        other => panic!("Expected WriteOperationsDisabled, got {:?}", other),
    }

    // 2. Enable write operations in policy and verify approval gate staging
    let custom_gate = ApprovalGate::new(WriteProtectionPolicy {
        write_operations_enabled: true,
        require_two_person_rule: true,
    });
    let enabled_adapter = AnsibleAdapter::new(
        Arc::new(MockSubprocessRunner::new()),
        PlaybookRegistry::with_default_catalog("/etc/idf/playbooks"),
        custom_gate.clone(),
        Path::new("/etc/idf/inventory/hosts.ini"),
        Duration::from_secs(60),
    );

    // Submission stages as AwaitingApproval
    let staged = enabled_adapter
        .submit_job(write_req, &net)
        .expect("stage write job");
    assert_eq!(staged.state, JobState::AwaitingApproval);

    // 3. Two-person rule violation: operator cannot approve their own write job
    let self_approve = enabled_adapter.approve_and_execute_job(
        &staged.job_id,
        "operator_bob", // same as operator
        "Self sign-off",
        &net,
    );
    match self_approve {
        Err(AnsibleError::Approval(ApprovalError::TwoPersonRuleViolation { .. })) => {}
        other => panic!("Expected TwoPersonRuleViolation, got {:?}", other),
    }

    // 4. Authorized peer approval executes the job
    let approved = enabled_adapter
        .approve_and_execute_job(
            &staged.job_id,
            "peer_lead_sarah", // distinct reviewer
            "Approved change for maintenance window",
            &net,
        )
        .expect("authorized execution");

    assert_eq!(approved.state, JobState::Completed);
    assert!(approved.approval_status.is_approved());
}

#[test]
fn test_per_device_status_parsing() {
    let ansible_output = r#"
PLAY [Configure Access Switches] **********************************************
TASK [Configure VLAN] *********************************************************
changed: [sw-idf1a-01.hq.net]
ok: [sw-idf1a-02.hq.net]
fatal: [sw-idfwh-01.hq.net]: FAILED! => {"msg": "Connection timeout"}

PLAY RECAP *********************************************************************
sw-idf1a-01.hq.net         : ok=4    changed=1    unreachable=0    failed=0    skipped=0    rescued=0    ignored=0
sw-idf1a-02.hq.net         : ok=5    changed=0    unreachable=0    failed=0    skipped=1    rescued=0    ignored=0
sw-idf2b-01.hq.net         : ok=0    changed=0    unreachable=1    failed=0    skipped=0    rescued=0    ignored=0
sw-idfwh-01.hq.net         : ok=2    changed=0    unreachable=0    failed=1    skipped=0    rescued=0    ignored=0
"#;

    let statuses = StatusParser::parse_play_recap(ansible_output);
    assert_eq!(statuses.len(), 4);

    let sw1 = statuses.get("sw-idf1a-01.hq.net").unwrap();
    assert_eq!(sw1.state, DeviceExecutionState::Changed);
    assert_eq!(sw1.changed_count, 1);
    assert_eq!(sw1.ok_count, 4);

    let sw2 = statuses.get("sw-idf1a-02.hq.net").unwrap();
    assert_eq!(sw2.state, DeviceExecutionState::Ok);
    assert_eq!(sw2.changed_count, 0);

    let sw3 = statuses.get("sw-idf2b-01.hq.net").unwrap();
    assert_eq!(sw3.state, DeviceExecutionState::Unreachable);
    assert_eq!(sw3.unreachable_count, 1);

    let sw4 = statuses.get("sw-idfwh-01.hq.net").unwrap();
    assert_eq!(sw4.state, DeviceExecutionState::Failed);
    assert_eq!(sw4.failed_count, 1);
}

#[test]
fn test_post_change_verification() {
    let net = generate_mock_infrastructure();
    let sw = net.find_switch("sw-idf1a-01").unwrap();

    let mut status_map = HashMap::new();
    let mut dev_status = DeviceJobStatus::new("sw-idf1a-01.hq.net");
    dev_status.state = DeviceExecutionState::Ok;
    dev_status.ok_count = 3;
    status_map.insert("sw-idf1a-01.hq.net".to_string(), dev_status);

    // 1. Live change verification with normal switch
    let report = PostChangeVerifier::verify(std::slice::from_ref(sw), &status_map, false);
    // sw-idf1a-01 in mock data has 1 err-disabled port (BPDU guard), so interface health check produces Warning
    assert_eq!(report.status, VerificationStatus::Warning);
    assert!(!report.checks.is_empty());

    // 2. Dry run skips verification
    let dry_report = PostChangeVerifier::verify(std::slice::from_ref(sw), &status_map, true);
    assert_eq!(dry_report.status, VerificationStatus::Skipped);
}

#[test]
fn test_audit_history_and_secret_redaction() {
    let net = generate_mock_infrastructure();
    let adapter = AnsibleAdapter::new_mock("/etc/idf/playbooks");

    let mut req = JobRequest::dry_run(
        "backup_cisco_config.yml",
        TargetSelection::Single("sw-idf1a-01.hq.net".to_string()),
        "auditor_jen",
    );
    req.extra_vars.insert(
        "ansible_password".to_string(),
        "VaultSecretKey99".to_string(),
    );

    let entry = adapter.submit_job(req, &net).expect("submit job");

    // 1. Audit trail persistence
    let recorded = adapter
        .audit_trail
        .find_by_job_id(&entry.job_id)
        .expect("find job");
    assert_eq!(recorded.operator, "auditor_jen");
    assert_eq!(recorded.playbook_id, "pb-backup-cisco");

    // 2. Secret redaction in audit record
    assert!(!recorded.sanitized_command.contains("VaultSecretKey99"));
    assert!(recorded.sanitized_command.contains("[REDACTED]"));

    // 3. Device query filter
    let dev_entries = adapter.audit_trail.find_by_device("sw-idf1a-01.hq.net");
    assert!(!dev_entries.is_empty());
}

#[test]
fn test_ansible_timeout_handling() {
    let net = generate_mock_infrastructure();
    let mock_runner = Arc::new(MockSubprocessRunner::new());
    mock_runner.set_timeout(Some(Duration::from_secs(30)));

    let adapter = AnsibleAdapter::new(
        mock_runner,
        PlaybookRegistry::with_default_catalog("/etc/idf/playbooks"),
        ApprovalGate::new(WriteProtectionPolicy::default()),
        Path::new("/etc/idf/inventory/hosts.ini"),
        Duration::from_secs(30),
    );

    let req = JobRequest::dry_run(
        "verify_interfaces.yml",
        TargetSelection::Single("sw-idf1a-01.hq.net".to_string()),
        "operator",
    );

    let err = adapter.submit_job(req, &net).unwrap_err();
    match err {
        AnsibleError::Timeout { duration, .. } => {
            assert_eq!(duration, Duration::from_secs(30));
        }
        other => panic!("Expected Timeout, got {:?}", other),
    }

    // Verify audit entry state is TimedOut
    let entries = adapter.audit_trail.all_entries();
    assert_eq!(entries.last().unwrap().state, JobState::TimedOut);
}

#[test]
fn test_tui_ansible_jobs_tab_navigation() {
    let net = generate_mock_infrastructure();
    let mut app = App::new(net);

    // 1. Direct switch to tab 6 (Ansible & Audit)
    app.handle_key(make_key(KeyCode::Char('6')));
    assert_eq!(app.active_tab, Tab::AnsibleJobs);

    // 2. Trigger dry-run verification with 'd'
    app.handle_key(make_key(KeyCode::Char('d')));
    assert!(app.notification.is_some());
    let notif = app.notification.clone().unwrap();
    assert_eq!(notif.level, crate::app::NotificationLevel::Success);
    assert!(notif.message.contains("Dry-run check completed"));

    // 3. Test safety gate with 'w' (write locked by default)
    app.handle_key(make_key(KeyCode::Char('w')));
    assert!(app.notification.is_some());
    let safety_notif = app.notification.clone().unwrap();
    assert_eq!(safety_notif.level, crate::app::NotificationLevel::Warning);
    assert!(safety_notif.message.contains("blocked by safety policy"));
}
