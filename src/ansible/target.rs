use crate::model::{NetworkInfrastructure, Switch};
use std::collections::HashSet;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TargetValidationError {
    #[error("Target list cannot be empty")]
    EmptyTargets,
    #[error("Invalid character in target '{target}': {reason}")]
    InvalidCharacter { target: String, reason: String },
    #[error("Target '{target}' exceeds maximum length of 253 characters")]
    TargetTooLong { target: String },
    #[error("Device '{target}' was not found in infrastructure inventory")]
    DeviceNotFound { target: String },
    #[error("Rack '{rack}' was not found in infrastructure inventory")]
    RackNotFound { rack: String },
    #[error("IDF Closet '{closet}' was not found in infrastructure inventory")]
    ClosetNotFound { closet: String },
    #[error("Target '{target}' is offline and cannot accept automation jobs")]
    DeviceOffline { target: String },
}

/// Criterion for selecting automation targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSelection {
    /// A single switch by hostname or device ID
    Single(String),
    /// Explicit list of hostnames or device IDs
    Multiple(Vec<String>),
    /// All switches in a specific rack (e.g. "rack-1a-01")
    Rack(String),
    /// All switches in a specific IDF closet (e.g. "closet-idf1a")
    Closet(String),
    /// All managed switches in the infrastructure
    AllManaged,
}

/// A validated, sanitized set of devices ready for Ansible execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedTargets {
    pub devices: Vec<Switch>,
    pub hostnames: Vec<String>,
    pub limit_arg: String,
}

impl ValidatedTargets {
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub fn count(&self) -> usize {
        self.devices.len()
    }
}

pub struct TargetValidator;

impl TargetValidator {
    /// Validate individual hostname or IP syntax for injection hazards.
    pub fn validate_syntax(target: &str) -> Result<(), TargetValidationError> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(TargetValidationError::EmptyTargets);
        }

        if trimmed.len() > 253 {
            return Err(TargetValidationError::TargetTooLong {
                target: trimmed.to_string(),
            });
        }

        // Strict rejection of shell interpolation, control, or traversal characters
        let forbidden = [
            '&', '|', ';', '$', '`', '>', '<', '(', ')', '{', '}', '\n', '\r', ' ', '\t', '\\',
            '\'', '"',
        ];
        for ch in forbidden {
            if trimmed.contains(ch) {
                return Err(TargetValidationError::InvalidCharacter {
                    target: trimmed.to_string(),
                    reason: format!("Contains forbidden character '{ch}'"),
                });
            }
        }

        // RFC 1123 compliant characters only: alphanumeric, dot, hyphen, underscore
        for ch in trimmed.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '_' && ch != ':' {
                return Err(TargetValidationError::InvalidCharacter {
                    target: trimmed.to_string(),
                    reason: format!("Contains illegal non-RFC character '{ch}'"),
                });
            }
        }

        Ok(())
    }

    /// Validate target selection against current network infrastructure inventory.
    pub fn validate(
        selection: &TargetSelection,
        net: &NetworkInfrastructure,
        allow_offline: bool,
    ) -> Result<ValidatedTargets, TargetValidationError> {
        let mut matched_switches: Vec<Switch> = Vec::new();
        let mut seen_ids = HashSet::new();

        match selection {
            TargetSelection::Single(target_str) => {
                Self::validate_syntax(target_str)?;
                let sw = Self::find_switch_by_identifier(net, target_str).ok_or_else(|| {
                    TargetValidationError::DeviceNotFound {
                        target: target_str.clone(),
                    }
                })?;
                matched_switches.push(sw.clone());
            }

            TargetSelection::Multiple(targets) => {
                if targets.is_empty() {
                    return Err(TargetValidationError::EmptyTargets);
                }
                for t in targets {
                    Self::validate_syntax(t)?;
                    let sw = Self::find_switch_by_identifier(net, t).ok_or_else(|| {
                        TargetValidationError::DeviceNotFound { target: t.clone() }
                    })?;
                    if seen_ids.insert(sw.id.clone()) {
                        matched_switches.push(sw.clone());
                    }
                }
            }

            TargetSelection::Rack(rack_id) => {
                Self::validate_syntax(rack_id)?;
                let mut found_rack = false;
                for closet in &net.closets {
                    for rack in &closet.racks {
                        if rack.id == *rack_id || rack.name.eq_ignore_ascii_case(rack_id) {
                            found_rack = true;
                            for sw in &rack.switches {
                                if seen_ids.insert(sw.id.clone()) {
                                    matched_switches.push(sw.clone());
                                }
                            }
                        }
                    }
                }
                if !found_rack {
                    return Err(TargetValidationError::RackNotFound {
                        rack: rack_id.clone(),
                    });
                }
            }

            TargetSelection::Closet(closet_id) => {
                Self::validate_syntax(closet_id)?;
                let closet = net
                    .closets
                    .iter()
                    .find(|c| c.id == *closet_id || c.name.eq_ignore_ascii_case(closet_id))
                    .ok_or_else(|| TargetValidationError::ClosetNotFound {
                        closet: closet_id.clone(),
                    })?;

                for rack in &closet.racks {
                    for sw in &rack.switches {
                        if seen_ids.insert(sw.id.clone()) {
                            matched_switches.push(sw.clone());
                        }
                    }
                }
            }

            TargetSelection::AllManaged => {
                for closet in &net.closets {
                    for rack in &closet.racks {
                        for sw in &rack.switches {
                            if seen_ids.insert(sw.id.clone()) {
                                matched_switches.push(sw.clone());
                            }
                        }
                    }
                }
            }
        }

        if matched_switches.is_empty() {
            return Err(TargetValidationError::EmptyTargets);
        }

        if !allow_offline {
            for sw in &matched_switches {
                if sw.status == crate::model::DeviceStatus::Offline {
                    return Err(TargetValidationError::DeviceOffline {
                        target: sw.hostname.clone(),
                    });
                }
            }
        }

        let hostnames: Vec<String> = matched_switches
            .iter()
            .map(|s| s.hostname.clone())
            .collect();
        let limit_arg = hostnames.join(",");

        Ok(ValidatedTargets {
            devices: matched_switches,
            hostnames,
            limit_arg,
        })
    }

    fn find_switch_by_identifier<'a>(
        net: &'a NetworkInfrastructure,
        id: &str,
    ) -> Option<&'a Switch> {
        for closet in &net.closets {
            for rack in &closet.racks {
                for sw in &rack.switches {
                    if sw.id == id || sw.hostname == id || sw.ip == id {
                        return Some(sw);
                    }
                }
            }
        }
        None
    }
}
