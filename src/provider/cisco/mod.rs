pub mod fixtures;
pub mod parsers;

use std::sync::Arc;

use crate::model::{
    AdminStatus, DeviceStatus, Interface, OperStatus, PortMode, PowerSupplyStatus, Switch,
};
use crate::provider::cisco::parsers::*;
use crate::provider::concurrency::ConcurrencyLimiter;
use crate::provider::ssh::SshTransport;
use crate::provider::{DeviceTarget, ProviderError};

/// Documented Target Platform Details & Architectural Assumptions:
///
/// 1. Target Hardware Models:
///    - Cisco Catalyst 9300-48P (48-port Gigabit PoE+, 10G uplinks, StackWise-480)
///    - Cisco Catalyst 9300-24P (24-port Gigabit PoE+, 10G uplinks)
/// 2. Target Operating System & Version:
///    - Cisco IOS-XE Software Version 17.09.04a (Cupertino release train)
/// 3. Read-Only Policy:
///    - Only approved non-intrusive `show` diagnostic commands are sent.
///    - Configuration commands (e.g. `configure terminal`, `write memory`, `shutdown`) are strictly forbidden.
/// 4. Assumptions & Fallbacks:
///    - If switches run classic Cisco IOS 15.x or IOS-XE 16.x, commands are configurable via `CiscoDeviceProfile`.
///    - Dual power supplies (PSU-1 / PSU-2) and modular fan trays are parsed dynamically.
///    - PoE budget is extracted from `show power inline` (default 740W for 48P, 445W for 24P if absent).
#[derive(Debug, Clone)]
pub struct CiscoDeviceProfile {
    pub profile_name: String,
    pub show_version_cmd: String,
    pub show_interfaces_status_cmd: String,
    pub show_interfaces_cmd: String,
    pub show_environment_cmd: String,
    pub show_power_inline_cmd: String,
    pub show_cpu_cmd: String,
    pub show_memory_cmd: String,
    pub show_switch_cmd: String,
}

impl Default for CiscoDeviceProfile {
    fn default() -> Self {
        Self::catalyst_9300_ios_xe_17()
    }
}

impl CiscoDeviceProfile {
    /// Standard profile tailored to Cisco Catalyst 9300 running IOS-XE 17.9.4a.
    pub fn catalyst_9300_ios_xe_17() -> Self {
        Self {
            profile_name: "Cisco Catalyst 9300 (IOS-XE 17.09.04a)".to_string(),
            show_version_cmd: "show version".to_string(),
            show_interfaces_status_cmd: "show interfaces status".to_string(),
            show_interfaces_cmd: "show interfaces".to_string(),
            show_environment_cmd: "show environment all".to_string(),
            show_power_inline_cmd: "show power inline".to_string(),
            show_cpu_cmd: "show processes cpu sorted".to_string(),
            show_memory_cmd: "show processes memory".to_string(),
            show_switch_cmd: "show switch".to_string(),
        }
    }
}

/// Consolidated snapshot of switch telemetry collected via read-only SSH.
#[derive(Debug, Clone)]
pub struct SwitchTelemetrySnapshot {
    pub system_info: CiscoSystemInfo,
    pub interfaces: Vec<Interface>,
    pub environment: CiscoEnvironmentInfo,
    pub poe: CiscoPoeInfo,
    pub cpu_usage: u8,
    pub mem_usage: u8,
    pub stack_members: Vec<CiscoStackMemberInfo>,
}

/// Provider responsible for executing read-only Cisco switch queries and parsing data.
pub struct CiscoSwitchProvider {
    transport: Arc<dyn SshTransport>,
    profile: CiscoDeviceProfile,
    limiter: ConcurrencyLimiter,
}

impl CiscoSwitchProvider {
    pub fn new(transport: Arc<dyn SshTransport>, limiter: ConcurrencyLimiter) -> Self {
        Self {
            transport,
            profile: CiscoDeviceProfile::default(),
            limiter,
        }
    }

    pub fn with_profile(mut self, profile: CiscoDeviceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Fetch all telemetry from target switch with bounded concurrency.
    pub fn fetch_telemetry(
        &self,
        target: &DeviceTarget,
    ) -> Result<SwitchTelemetrySnapshot, ProviderError> {
        self.limiter
            .execute_bounded(target.timeouts.command_timeout, || {
                self.fetch_telemetry_internal(target)
            })
            .map_err(|e| ProviderError::Concurrency {
                target: target.hostname.clone(),
                source: e,
            })?
    }

    fn fetch_telemetry_internal(
        &self,
        target: &DeviceTarget,
    ) -> Result<SwitchTelemetrySnapshot, ProviderError> {
        // 1. `show version`
        let ver_out = self
            .transport
            .execute(target, &self.profile.show_version_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let system_info =
            parse_show_version(&ver_out.stdout).map_err(|e| ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            })?;

        // 2. `show interfaces status`
        let int_status_out = self
            .transport
            .execute(target, &self.profile.show_interfaces_status_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let int_statuses = parse_show_interfaces_status(&int_status_out.stdout).map_err(|e| {
            ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            }
        })?;

        // 3. `show interfaces` (detailed counters)
        let int_detail_out = self
            .transport
            .execute(target, &self.profile.show_interfaces_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let int_counters = parse_show_interfaces_detail(&int_detail_out.stdout).map_err(|e| {
            ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            }
        })?;

        // 4. `show power inline`
        let poe_out = self
            .transport
            .execute(target, &self.profile.show_power_inline_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let poe_info =
            parse_show_power_inline(&poe_out.stdout).map_err(|e| ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            })?;

        // 5. `show environment all`
        let env_out = self
            .transport
            .execute(target, &self.profile.show_environment_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let env_info =
            parse_show_environment(&env_out.stdout).map_err(|e| ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            })?;

        // 6. `show processes cpu sorted`
        let cpu_out = self
            .transport
            .execute(target, &self.profile.show_cpu_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let cpu_usage =
            parse_show_processes_cpu(&cpu_out.stdout).map_err(|e| ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            })?;

        // 7. `show processes memory`
        let mem_out = self
            .transport
            .execute(target, &self.profile.show_memory_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let mem_usage =
            parse_show_processes_memory(&mem_out.stdout).map_err(|e| ProviderError::Parse {
                target: target.hostname.clone(),
                source: e,
            })?;

        // 8. `show switch`
        let switch_out = self
            .transport
            .execute(target, &self.profile.show_switch_cmd)
            .map_err(|e| ProviderError::Ssh {
                target: target.hostname.clone(),
                source: e,
            })?;
        let stack_members = parse_show_switch(&switch_out.stdout).unwrap_or_default();
        // Correlate into domain Interface models
        let mut interfaces = Vec::with_capacity(int_statuses.len());
        for st in int_statuses {
            let counters = int_counters.get(&st.port);
            let poe_watts = poe_info.port_power.get(&st.port).copied().unwrap_or(0.0);
            let poe_enabled = poe_watts > 0.0;

            let oper_status = match st.status.to_lowercase().as_str() {
                "connected" => OperStatus::Up,
                "err-disabled" | "errdisabled" => OperStatus::ErrDisabled,
                "disabled" => OperStatus::Down,
                _ => OperStatus::Down,
            };

            let admin_status = if let Some(c) = counters {
                if c.admin_up {
                    AdminStatus::Up
                } else {
                    AdminStatus::Down
                }
            } else {
                AdminStatus::Up
            };

            let mode = if st.vlan.eq_ignore_ascii_case("trunk") {
                PortMode::Trunk
            } else if st.vlan.eq_ignore_ascii_case("routed") {
                PortMode::Routed
            } else {
                PortMode::Access
            };

            let (vlan, vlan_name) = if st.vlan.eq_ignore_ascii_case("trunk") {
                (1, "TRUNK".to_string())
            } else {
                let id = st.vlan.parse::<u16>().unwrap_or(1);
                (id, format!("VLAN_{id}"))
            };

            let desc = if let Some(c) = counters {
                if !c.description.is_empty() {
                    c.description.clone()
                } else if !st.name.is_empty() {
                    st.name.clone()
                } else {
                    "Unconfigured".to_string()
                }
            } else if !st.name.is_empty() {
                st.name.clone()
            } else {
                "Unconfigured".to_string()
            };

            let rx_bytes = counters.map(|c| c.rx_bytes).unwrap_or(0);
            let tx_bytes = counters.map(|c| c.tx_bytes).unwrap_or(0);
            let err_rx = counters.map(|c| c.errors_rx).unwrap_or(0);
            let err_tx = counters.map(|c| c.errors_tx).unwrap_or(0);
            let mtu = counters.map(|c| c.mtu).unwrap_or(1500);
            let mac_learned = counters.and_then(|c| c.mac_address.clone());

            interfaces.push(Interface {
                name: st.port,
                description: desc,
                admin_status,
                oper_status,
                mode,
                vlan,
                vlan_name,
                speed: st.speed,
                duplex: st.duplex,
                poe_enabled,
                poe_watts,
                rx_bytes,
                tx_bytes,
                errors_rx: err_rx,
                errors_tx: err_tx,
                mtu,
                mac_learned,
            });
        }

        Ok(SwitchTelemetrySnapshot {
            system_info,
            interfaces,
            environment: env_info,
            poe: poe_info,
            cpu_usage,
            mem_usage,
            stack_members,
        })
    }

    /// Read switch telemetry and synchronize into an existing domain `Switch` instance in place.
    pub fn sync_switch(
        &self,
        target: &DeviceTarget,
        switch: &mut Switch,
    ) -> Result<(), ProviderError> {
        let snapshot = self.fetch_telemetry(target)?;

        switch.hostname = snapshot.system_info.hostname;
        switch.model = snapshot.system_info.model;
        switch.serial = snapshot.system_info.serial;
        switch.os_version = snapshot.system_info.os_version;
        switch.uptime = snapshot.system_info.uptime;
        switch.mac = snapshot.system_info.mac;
        switch.cpu_usage = snapshot.cpu_usage;
        switch.mem_usage = snapshot.mem_usage;
        switch.temp_celsius = snapshot.environment.temp_celsius;
        switch.psu_1 = snapshot.environment.psu_1;
        switch.psu_2 = snapshot.environment.psu_2;
        switch.fan_status = snapshot.environment.fan_status;
        switch.poe_budget_watts = snapshot.poe.budget_watts;

        if let Some(m) = snapshot.stack_members.iter().find(|m| m.switch_num == 1) {
            switch.stack_role = m.role.clone();
        }

        let has_err_ports = snapshot
            .interfaces
            .iter()
            .any(|i| i.oper_status == OperStatus::ErrDisabled);
        let has_psu_fault =
            switch.psu_1 == PowerSupplyStatus::Fault || switch.psu_2 == PowerSupplyStatus::Fault;
        let has_high_temp = switch.temp_celsius > 50;

        if has_err_ports || has_psu_fault || has_high_temp {
            switch.status = DeviceStatus::Degraded;
        } else {
            switch.status = DeviceStatus::Online;
        }

        if !snapshot.interfaces.is_empty() {
            switch.interfaces = snapshot.interfaces;
        }

        Ok(())
    }
}
