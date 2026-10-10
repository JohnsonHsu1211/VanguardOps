use crate::model::{FanStatus, PowerSupplyStatus, StackRole};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("Missing required field in Cisco output: {0}")]
    MissingField(String),
    #[error("Failed to parse value for field '{field}': {value}")]
    InvalidValue { field: String, value: String },
    #[error("Unrecognized or empty command output")]
    EmptyOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiscoSystemInfo {
    pub hostname: String,
    pub model: String,
    pub serial: String,
    pub os_version: String,
    pub uptime: String,
    pub mac: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiscoInterfaceStatus {
    pub port: String,
    pub name: String,
    pub status: String,
    pub vlan: String,
    pub duplex: String,
    pub speed: String,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CiscoInterfaceCounters {
    pub port: String,
    pub description: String,
    pub admin_up: bool,
    pub oper_up: bool,
    pub mtu: u16,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub errors_rx: u32,
    pub errors_tx: u32,
    pub speed: String,
    pub duplex: String,
    pub mac_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiscoEnvironmentInfo {
    pub temp_celsius: u8,
    pub psu_1: PowerSupplyStatus,
    pub psu_2: PowerSupplyStatus,
    pub fan_status: FanStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CiscoPoeInfo {
    pub budget_watts: f32,
    pub used_watts: f32,
    pub port_power: HashMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiscoStackMemberInfo {
    pub switch_num: u8,
    pub role: StackRole,
    pub mac: String,
    pub state: String,
}

/// Parses the output of `show version` on Cisco IOS-XE 17.9.4a and Catalyst 9300.
pub fn parse_show_version(output: &str) -> Result<CiscoSystemInfo, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut hostname = String::new();
    let mut model = String::new();
    let mut serial = String::new();
    let mut os_version = String::new();
    let mut uptime = String::new();
    let mut mac = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // 1. OS version (e.g. "Cisco IOS XE Software, Version 17.09.04a")
        if os_version.is_empty() && trimmed.contains("Cisco IOS XE Software") {
            if let Some(pos) = trimmed.find("Version ") {
                os_version = trimmed[pos + 8..].trim().to_string();
            } else {
                os_version = trimmed.to_string();
            }
        } else if os_version.is_empty()
            && trimmed.contains("IOS (tm)")
            && let Some(pos) = trimmed.find("Version ")
        {
            os_version = trimmed[pos + 8..]
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
        }

        // 2. Uptime and Hostname (e.g. "sw-idf1a-01 uptime is 45 days, 2 hours, 11 minutes")
        if uptime.is_empty() && trimmed.contains(" uptime is ") {
            let parts: Vec<&str> = trimmed.split(" uptime is ").collect();
            if parts.len() == 2 {
                hostname = parts[0].trim().to_string();
                uptime = parts[1].trim().to_string();
            }
        }

        // 3. Model number (e.g. "Model Number : C9300-48P" or "cisco C9300-48P (X86)")
        if model.is_empty() {
            if trimmed.starts_with("Model Number") {
                if let Some(colon) = trimmed.find(':') {
                    model = trimmed[colon + 1..].trim().to_string();
                }
            } else if trimmed.starts_with("cisco ") && trimmed.contains(" processor") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    model = parts[1].to_string();
                }
            }
        }

        // 4. System Serial (e.g. "System Serial Number : FOC2419S3KL" or "Processor board ID ...")
        if serial.is_empty() {
            if trimmed.starts_with("System Serial Number") {
                if let Some(colon) = trimmed.find(':') {
                    serial = trimmed[colon + 1..].trim().to_string();
                }
            } else if trimmed.starts_with("Processor board ID ") {
                serial = trimmed
                    .replace("Processor board ID ", "")
                    .trim()
                    .to_string();
            }
        }

        // 5. Base Ethernet MAC Address (e.g. "Base Ethernet MAC Address : f4:0f:1b:2c:11:01")
        if mac.is_empty()
            && trimmed.starts_with("Base Ethernet MAC Address")
            && let Some(colon) = trimmed.find(':')
        {
            mac = trimmed[colon + 1..].trim().to_string();
        }
    }

    if model.is_empty() {
        return Err(ParseError::MissingField("model".to_string()));
    }
    if serial.is_empty() {
        return Err(ParseError::MissingField("serial".to_string()));
    }

    Ok(CiscoSystemInfo {
        hostname: if hostname.is_empty() {
            "switch".to_string()
        } else {
            hostname
        },
        model,
        serial,
        os_version: if os_version.is_empty() {
            "IOS-XE 17.x".to_string()
        } else {
            os_version
        },
        uptime: if uptime.is_empty() {
            "Unknown".to_string()
        } else {
            uptime
        },
        mac: if mac.is_empty() {
            "00:00:00:00:00:00".to_string()
        } else {
            mac
        },
    })
}

/// Parses the output of `show interfaces status` on Cisco Catalyst 9300.
pub fn parse_show_interfaces_status(output: &str) -> Result<Vec<CiscoInterfaceStatus>, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut list = Vec::new();
    let mut header_seen = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Port ") && trimmed.contains("Status") {
            header_seen = true;
            continue;
        }

        if !header_seen || trimmed.is_empty() {
            continue;
        }

        // Fixed columns in `show interfaces status`:
        if line.len() >= 30 {
            let port = line
                .get(0..10.min(line.len()))
                .unwrap_or("")
                .trim()
                .to_string();
            let name = if line.len() >= 29 {
                line[10..29].trim().to_string()
            } else {
                String::new()
            };
            let status = if line.len() >= 42 {
                line[29..42].trim().to_string()
            } else {
                String::new()
            };
            let vlan = if line.len() >= 53 {
                line[42..53].trim().to_string()
            } else {
                "1".to_string()
            };
            let duplex = if line.len() >= 61 {
                line[53..61].trim().to_string()
            } else {
                "auto".to_string()
            };
            let speed = if line.len() >= 68 {
                line[61..68].trim().to_string()
            } else {
                "auto".to_string()
            };
            let media_type = if line.len() > 68 {
                line[68..].trim().to_string()
            } else {
                String::new()
            };

            if !port.is_empty() {
                list.push(CiscoInterfaceStatus {
                    port,
                    name,
                    status,
                    vlan,
                    duplex,
                    speed,
                    media_type,
                });
            }
        } else {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 6 {
                list.push(CiscoInterfaceStatus {
                    port: parts[0].to_string(),
                    name: String::new(),
                    status: parts[1].to_string(),
                    vlan: parts[2].to_string(),
                    duplex: parts[3].to_string(),
                    speed: parts[4].to_string(),
                    media_type: parts.get(5).unwrap_or(&"").to_string(),
                });
            }
        }
    }

    Ok(list)
}

/// Parses detailed `show interfaces` counters and configurations.
pub fn parse_show_interfaces_detail(
    output: &str,
) -> Result<HashMap<String, CiscoInterfaceCounters>, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut map = HashMap::new();
    let sections = output.split("\n\n");

    for section in sections {
        let trimmed = section.trim();
        if trimmed.is_empty() {
            continue;
        }

        let first_line = trimmed.lines().next().unwrap_or("");
        if !first_line.contains(" is ") {
            continue;
        }

        let port_part = first_line.split_whitespace().next().unwrap_or("");
        if port_part.is_empty() {
            continue;
        }
        let port_name = port_part.to_string();

        let admin_up = !first_line.contains("administratively down");
        let oper_up = first_line.contains("line protocol is up");

        let mut desc = String::new();
        let mut mtu = 1500;
        let mut rx_bytes = 0u64;
        let mut tx_bytes = 0u64;
        let mut rx_pkts = 0u64;
        let mut tx_pkts = 0u64;
        let mut err_rx = 0u32;
        let mut err_tx = 0u32;
        let mut duplex = "Full".to_string();
        let speed = "1000 Mbps".to_string();
        let mut mac = None;

        for line in trimmed.lines() {
            let l = line.trim();
            if l.starts_with("Description:") {
                desc = l.replace("Description:", "").trim().to_string();
            } else if l.contains("MTU ") && l.contains(" bytes") {
                if let Some(pos) = l.find("MTU ") {
                    let sub = &l[pos + 4..];
                    if let Some(end) = sub.find(' ') {
                        mtu = sub[..end].parse().unwrap_or(1500);
                    }
                }
            } else if l.contains("packets input,") && l.contains("bytes") {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 5 {
                    rx_pkts = parts[0].parse().unwrap_or(0);
                    rx_bytes = parts[3].parse().unwrap_or(0);
                }
            } else if l.contains("packets output,") && l.contains("bytes") {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 5 {
                    tx_pkts = parts[0].parse().unwrap_or(0);
                    tx_bytes = parts[3].parse().unwrap_or(0);
                }
            } else if l.contains("input errors,") {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if !parts.is_empty() {
                    err_rx = parts[0].parse().unwrap_or(0);
                }
            } else if l.contains("output errors,") {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if !parts.is_empty() {
                    err_tx = parts[0].parse().unwrap_or(0);
                }
            } else if l.contains("address is ") {
                if let Some(pos) = l.find("address is ") {
                    let sub = &l[pos + 11..];
                    let addr = sub.split_whitespace().next().unwrap_or("");
                    if !addr.is_empty() {
                        mac = Some(addr.to_string());
                    }
                }
            } else if (l.contains("duplex,") || l.contains("-duplex"))
                && l.to_lowercase().contains("half")
            {
                duplex = "Half".to_string();
            }
        }

        map.insert(
            port_name.clone(),
            CiscoInterfaceCounters {
                port: port_name,
                description: desc,
                admin_up,
                oper_up,
                mtu,
                rx_bytes,
                tx_bytes,
                rx_packets: rx_pkts,
                tx_packets: tx_pkts,
                errors_rx: err_rx,
                errors_tx: err_tx,
                speed,
                duplex,
                mac_address: mac,
            },
        );
    }

    Ok(map)
}

/// Parses `show environment all` or `show environment` output.
pub fn parse_show_environment(output: &str) -> Result<CiscoEnvironmentInfo, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut temp = 35u8;
    let mut psu1 = PowerSupplyStatus::Ok;
    let mut psu2 = PowerSupplyStatus::NotPresent;
    let mut fan = FanStatus::Normal;

    let mut in_psu_section = false;
    let mut in_fan_section = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Power Supply:") {
            in_psu_section = true;
            in_fan_section = false;
            continue;
        } else if trimmed.starts_with("Fan:") {
            in_psu_section = false;
            in_fan_section = true;
            continue;
        }

        // Temperature (e.g. "Hotspot Switch 1 Normal 42 Celsius" or "ASIC Switch 1 Normal 48 Celsius")
        if trimmed.contains("Celsius") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            for (idx, &part) in parts.iter().enumerate() {
                if part.eq_ignore_ascii_case("celsius")
                    && idx > 0
                    && let Ok(val) = parts[idx - 1].parse::<u8>()
                    && val > temp
                {
                    temp = val;
                }
            }
        }

        // PSUs (e.g. "1A PWR-C1-715WAC-P LIT24100XYZ OK ...")
        if in_psu_section {
            let upper = trimmed.to_uppercase();
            if upper.starts_with("1A") || upper.starts_with("PS1") || upper.starts_with("PSU1") {
                if upper.contains("FAULT") || upper.contains("FAIL") || upper.contains("BAD") {
                    psu1 = PowerSupplyStatus::Fault;
                } else if upper.contains("OK") || upper.contains("GOOD") {
                    psu1 = PowerSupplyStatus::Ok;
                }
            } else if upper.starts_with("1B")
                || upper.starts_with("PS2")
                || upper.starts_with("PSU2")
            {
                if upper.contains("NOT PRESENT")
                    || upper.contains("ABSENT")
                    || upper.contains("EMPTY")
                {
                    psu2 = PowerSupplyStatus::NotPresent;
                } else if upper.contains("FAULT") || upper.contains("FAIL") || upper.contains("BAD")
                {
                    psu2 = PowerSupplyStatus::Fault;
                } else if upper.contains("OK") || upper.contains("GOOD") {
                    psu2 = PowerSupplyStatus::RedundantOk;
                }
            }
        }

        // Fans
        if in_fan_section {
            let upper = trimmed.to_uppercase();
            if upper.contains("FAULT") || upper.contains("FAIL") {
                fan = FanStatus::Failed;
            } else if upper.contains("WARN") {
                fan = FanStatus::Warning;
            }
        }
    }

    Ok(CiscoEnvironmentInfo {
        temp_celsius: temp,
        psu_1: psu1,
        psu_2: psu2,
        fan_status: fan,
    })
}

/// Parses `show power inline` output for PoE budget and port draw.
pub fn parse_show_power_inline(output: &str) -> Result<CiscoPoeInfo, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut budget = 0.0f32;
    let mut used = 0.0f32;
    let mut port_power = HashMap::new();

    let mut in_budget_table = false;
    let mut in_port_table = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Module") && trimmed.contains("Available") {
            in_budget_table = true;
            in_port_table = false;
            continue;
        } else if trimmed.starts_with("Interface") && trimmed.contains("Admin") {
            in_budget_table = false;
            in_port_table = true;
            continue;
        }

        if in_budget_table && !trimmed.starts_with('-') && !trimmed.starts_with('(') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4 {
                budget = parts[1].parse().unwrap_or(0.0);
                used = parts[2].parse().unwrap_or(0.0);
            }
        }

        if in_port_table && !trimmed.starts_with('-') && !trimmed.starts_with('(') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4 {
                let iface = parts[0].to_string();
                if let Ok(watts) = parts[3].parse::<f32>() {
                    port_power.insert(iface, watts);
                }
            }
        }
    }

    Ok(CiscoPoeInfo {
        budget_watts: budget,
        used_watts: used,
        port_power,
    })
}

/// Parses CPU utilization percentage from `show processes cpu`.
pub fn parse_show_processes_cpu(output: &str) -> Result<u8, ParseError> {
    for line in output.lines() {
        if line.contains("CPU utilization for five seconds:")
            && let Some(pos) = line.find("five seconds:")
        {
            let sub = &line[pos + 13..];
            let pct_str = sub.split('%').next().unwrap_or("").trim();
            if let Ok(val) = pct_str.parse::<u8>() {
                return Ok(val);
            }
        }
    }
    Ok(15)
}

/// Parses memory utilization percentage from `show processes memory`.
pub fn parse_show_processes_memory(output: &str) -> Result<u8, ParseError> {
    for line in output.lines() {
        if line.contains("Processor Pool Total:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                let total: u64 = parts[3].parse().unwrap_or(0);
                let used: u64 = parts[5].parse().unwrap_or(0);
                if total > 0 {
                    let pct = ((used as f64 / total as f64) * 100.0) as u8;
                    return Ok(pct);
                }
            }
        }
    }
    Ok(50)
}

/// Parses `show switch` stack topology output.
pub fn parse_show_switch(output: &str) -> Result<Vec<CiscoStackMemberInfo>, ParseError> {
    if output.trim().is_empty() {
        return Err(ParseError::EmptyOutput);
    }

    let mut members = Vec::new();
    let mut table_started = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Switch#") && trimmed.contains("Role") {
            table_started = true;
            continue;
        }

        if !table_started || trimmed.starts_with('-') || trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 5 {
            let num_str = parts[0].trim_start_matches('*');
            let member_id: u8 = num_str.parse().unwrap_or(1);
            let role_str = parts[1].to_lowercase();
            let role = if role_str.contains("active") || role_str.contains("master") {
                StackRole::Master
            } else if role_str.contains("standby") {
                StackRole::Standby
            } else {
                StackRole::Member(member_id)
            };

            let mac = parts[2].to_string();
            let state = parts.get(5).unwrap_or(&"Ready").to_string();

            members.push(CiscoStackMemberInfo {
                switch_num: member_id,
                role,
                mac,
                state,
            });
        }
    }

    Ok(members)
}
