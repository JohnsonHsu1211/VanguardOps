use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Online,
    Degraded,
    Offline,
}

impl DeviceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceStatus::Online => "ONLINE",
            DeviceStatus::Degraded => "DEGRADED",
            DeviceStatus::Offline => "OFFLINE",
        }
    }
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminStatus {
    Up,
    Down,
}

impl AdminStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdminStatus::Up => "up",
            AdminStatus::Down => "down",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperStatus {
    Up,
    Down,
    ErrDisabled,
    Testing,
}

impl OperStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperStatus::Up => "connected",
            OperStatus::Down => "notconnect",
            OperStatus::ErrDisabled => "err-disabled",
            OperStatus::Testing => "testing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortMode {
    Access,
    Trunk,
    Routed,
}

impl PortMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PortMode::Access => "access",
            PortMode::Trunk => "trunk",
            PortMode::Routed => "routed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSupplyStatus {
    Ok,
    Fault,
    RedundantOk,
    NotPresent,
}

impl PowerSupplyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerSupplyStatus::Ok => "Normal",
            PowerSupplyStatus::Fault => "FAULT",
            PowerSupplyStatus::RedundantOk => "Redundant OK",
            PowerSupplyStatus::NotPresent => "Absent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanStatus {
    Normal,
    Warning,
    Failed,
}

impl FanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FanStatus::Normal => "OK",
            FanStatus::Warning => "WARN",
            FanStatus::Failed => "FAIL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackRole {
    Standalone,
    Master,
    Standby,
    Member(u8),
}

impl StackRole {
    pub fn as_str(&self) -> String {
        match self {
            StackRole::Standalone => "Standalone".to_string(),
            StackRole::Master => "Stack Master (1)".to_string(),
            StackRole::Standby => "Stack Standby (2)".to_string(),
            StackRole::Member(n) => format!("Stack Member ({n})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub name: String,
    pub description: String,
    pub admin_status: AdminStatus,
    pub oper_status: OperStatus,
    pub mode: PortMode,
    pub vlan: u16,
    pub vlan_name: String,
    pub speed: String,
    pub duplex: String,
    pub poe_enabled: bool,
    pub poe_watts: f32,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub errors_rx: u32,
    pub errors_tx: u32,
    pub mtu: u16,
    pub mac_learned: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    pub id: String,
    pub hostname: String,
    pub ip: String,
    pub mac: String,
    pub vendor: String,
    pub model: String,
    pub serial: String,
    pub os_version: String,
    pub uptime: String,
    pub cpu_usage: u8,
    pub mem_usage: u8,
    pub temp_celsius: u8,
    pub psu_1: PowerSupplyStatus,
    pub psu_2: PowerSupplyStatus,
    pub fan_status: FanStatus,
    pub stack_role: StackRole,
    pub status: DeviceStatus,
    pub rack_unit: String,
    pub poe_budget_watts: f32,
    pub interfaces: Vec<Interface>,
}

impl Switch {
    pub fn active_ports_count(&self) -> usize {
        self.interfaces
            .iter()
            .filter(|i| i.oper_status == OperStatus::Up)
            .count()
    }

    pub fn err_ports_count(&self) -> usize {
        self.interfaces
            .iter()
            .filter(|i| i.oper_status == OperStatus::ErrDisabled)
            .count()
    }

    pub fn total_poe_draw(&self) -> f32 {
        self.interfaces.iter().map(|i| i.poe_watts).sum()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rack {
    pub id: String,
    pub name: String,
    pub ru_capacity: u8,
    pub switches: Vec<Switch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdfCloset {
    pub id: String,
    pub name: String,
    pub building: String,
    pub floor: String,
    pub ambient_temp_celsius: f32,
    pub ups_status: String,
    pub racks: Vec<Rack>,
}

impl IdfCloset {
    pub fn total_switches(&self) -> usize {
        self.racks.iter().map(|r| r.switches.len()).sum()
    }

    pub fn status(&self) -> DeviceStatus {
        let mut has_degraded = false;
        for rack in &self.racks {
            for sw in &rack.switches {
                match sw.status {
                    DeviceStatus::Offline => return DeviceStatus::Offline,
                    DeviceStatus::Degraded => has_degraded = true,
                    DeviceStatus::Online => {}
                }
            }
        }
        if has_degraded {
            DeviceStatus::Degraded
        } else {
            DeviceStatus::Online
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARN",
            AlertSeverity::Critical => "CRIT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub id: usize,
    pub timestamp: String,
    pub severity: AlertSeverity,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkInfrastructure {
    pub closets: Vec<IdfCloset>,
    pub alerts: Vec<Alert>,
}

impl NetworkInfrastructure {
    pub fn total_closets(&self) -> usize {
        self.closets.len()
    }

    pub fn total_switches(&self) -> usize {
        self.closets.iter().map(|c| c.total_switches()).sum()
    }

    pub fn total_interfaces(&self) -> usize {
        self.closets
            .iter()
            .flat_map(|c| &c.racks)
            .flat_map(|r| &r.switches)
            .map(|s| s.interfaces.len())
            .sum()
    }

    pub fn active_interfaces(&self) -> usize {
        self.closets
            .iter()
            .flat_map(|c| &c.racks)
            .flat_map(|r| &r.switches)
            .flat_map(|s| &s.interfaces)
            .filter(|i| i.oper_status == OperStatus::Up)
            .count()
    }

    pub fn err_interfaces(&self) -> usize {
        self.closets
            .iter()
            .flat_map(|c| &c.racks)
            .flat_map(|r| &r.switches)
            .flat_map(|s| &s.interfaces)
            .filter(|i| i.oper_status == OperStatus::ErrDisabled)
            .count()
    }

    pub fn total_poe_draw(&self) -> f32 {
        self.closets
            .iter()
            .flat_map(|c| &c.racks)
            .flat_map(|r| &r.switches)
            .map(|s| s.total_poe_draw())
            .sum()
    }

    pub fn total_poe_budget(&self) -> f32 {
        self.closets
            .iter()
            .flat_map(|c| &c.racks)
            .flat_map(|r| &r.switches)
            .map(|s| s.poe_budget_watts)
            .sum()
    }

    pub fn find_switch(&self, switch_id: &str) -> Option<&Switch> {
        for closet in &self.closets {
            for rack in &closet.racks {
                for sw in &rack.switches {
                    if sw.id == switch_id {
                        return Some(sw);
                    }
                }
            }
        }
        None
    }
}
