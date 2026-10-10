use crate::model::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard = 0,
    Inventory = 1,
    SwitchDetail = 2,
    Interfaces = 3,
    Alerts = 4,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Dashboard,
        Tab::Inventory,
        Tab::SwitchDetail,
        Tab::Interfaces,
        Tab::Alerts,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "1: Dashboard",
            Tab::Inventory => "2: IDF Inventory",
            Tab::SwitchDetail => "3: Switch Details",
            Tab::Interfaces => "4: Interfaces",
            Tab::Alerts => "5: Alerts & Logs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceFilter {
    All,
    UpOnly,
    DownOnly,
    ErrDisabledOnly,
    PoeOnly,
}

impl InterfaceFilter {
    pub fn cycle(&self) -> Self {
        match self {
            InterfaceFilter::All => InterfaceFilter::UpOnly,
            InterfaceFilter::UpOnly => InterfaceFilter::DownOnly,
            InterfaceFilter::DownOnly => InterfaceFilter::ErrDisabledOnly,
            InterfaceFilter::ErrDisabledOnly => InterfaceFilter::PoeOnly,
            InterfaceFilter::PoeOnly => InterfaceFilter::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InterfaceFilter::All => "All Ports",
            InterfaceFilter::UpOnly => "Connected (Up)",
            InterfaceFilter::DownOnly => "Not Connect (Down)",
            InterfaceFilter::ErrDisabledOnly => "Err-Disabled Only",
            InterfaceFilter::PoeOnly => "PoE Active Only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceSortKey {
    PortName,
    Status,
    Vlan,
    Speed,
    Poe,
    Errors,
    Traffic,
}

impl InterfaceSortKey {
    pub fn cycle(&self) -> Self {
        match self {
            InterfaceSortKey::PortName => InterfaceSortKey::Status,
            InterfaceSortKey::Status => InterfaceSortKey::Vlan,
            InterfaceSortKey::Vlan => InterfaceSortKey::Speed,
            InterfaceSortKey::Speed => InterfaceSortKey::Poe,
            InterfaceSortKey::Poe => InterfaceSortKey::Errors,
            InterfaceSortKey::Errors => InterfaceSortKey::Traffic,
            InterfaceSortKey::Traffic => InterfaceSortKey::PortName,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InterfaceSortKey::PortName => "Port",
            InterfaceSortKey::Status => "Status",
            InterfaceSortKey::Vlan => "VLAN",
            InterfaceSortKey::Speed => "Speed",
            InterfaceSortKey::Poe => "PoE (W)",
            InterfaceSortKey::Errors => "Errors",
            InterfaceSortKey::Traffic => "Traffic (Rx)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatusFilter {
    All,
    OnlineOnly,
    IssuesOnly,
}

impl DeviceStatusFilter {
    pub fn cycle(&self) -> Self {
        match self {
            DeviceStatusFilter::All => DeviceStatusFilter::OnlineOnly,
            DeviceStatusFilter::OnlineOnly => DeviceStatusFilter::IssuesOnly,
            DeviceStatusFilter::IssuesOnly => DeviceStatusFilter::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceStatusFilter::All => "All Devices",
            DeviceStatusFilter::OnlineOnly => "Online Only",
            DeviceStatusFilter::IssuesOnly => "Issues Only (Degraded/Offline)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    Closet(usize),
    Rack(usize, usize),
    Switch(usize, usize, usize),
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub subtitle: String,
    pub closet_idx: usize,
    pub rack_idx: usize,
    pub switch_idx: usize,
    pub interface_idx: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
}

pub struct App {
    pub net: NetworkInfrastructure,
    pub active_tab: Tab,
    pub should_quit: bool,

    // Tree navigation state
    pub expanded_closets: Vec<bool>,
    pub expanded_racks: Vec<Vec<bool>>,
    pub tree_cursor: usize,
    pub selected_switch: (usize, usize, usize), // (closet_idx, rack_idx, switch_idx)
    pub status_filter: DeviceStatusFilter,

    // Interface table state
    pub selected_interface_idx: usize,
    pub interface_sort: InterfaceSortKey,
    pub interface_sort_asc: bool,
    pub interface_filter: InterfaceFilter,
    pub port_detail_modal_open: bool,

    // Search state
    pub search_active: bool,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_cursor: usize,

    // Modals & Error handling
    pub help_modal_open: bool,
    pub error_modal: Option<String>,
    pub notification: Option<Notification>,

    // Dashboard scroll
    pub dashboard_alert_cursor: usize,

    // Cisco Provider (Milestone 2)
    pub cisco_provider: std::sync::Arc<crate::provider::cisco::CiscoSwitchProvider>,
}

impl App {
    pub fn new(net: NetworkInfrastructure) -> Self {
        let closet_count = net.closets.len();
        let expanded_closets = vec![true; closet_count];
        let mut expanded_racks = Vec::with_capacity(closet_count);
        for closet in &net.closets {
            expanded_racks.push(vec![true; closet.racks.len()]);
        }

        let transport =
            std::sync::Arc::new(crate::provider::ssh::mock::MockSshTransport::new_cisco_c9300());
        let limiter = crate::provider::concurrency::ConcurrencyLimiter::new(4);
        let cisco_provider = std::sync::Arc::new(crate::provider::cisco::CiscoSwitchProvider::new(
            transport, limiter,
        ));

        let mut app = Self {
            net,
            active_tab: Tab::Dashboard,
            should_quit: false,
            expanded_closets,
            expanded_racks,
            tree_cursor: 0,
            selected_switch: (0, 0, 0),
            status_filter: DeviceStatusFilter::All,
            selected_interface_idx: 0,
            interface_sort: InterfaceSortKey::PortName,
            interface_sort_asc: true,
            interface_filter: InterfaceFilter::All,
            port_detail_modal_open: false,
            search_active: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            help_modal_open: false,
            error_modal: None,
            notification: Some(Notification {
                message: "System initialized. Press ? for keyboard shortcuts.".to_string(),
                level: NotificationLevel::Info,
            }),
            dashboard_alert_cursor: 0,
            cisco_provider,
        };

        // Select the first valid switch if available
        if let Some((c, r, s)) = app.find_first_switch() {
            app.selected_switch = (c, r, s);
        }

        app
    }

    fn find_first_switch(&self) -> Option<(usize, usize, usize)> {
        for (c, closet) in self.net.closets.iter().enumerate() {
            for (r, rack) in closet.racks.iter().enumerate() {
                if !rack.switches.is_empty() {
                    return Some((c, r, 0));
                }
            }
        }
        None
    }

    pub fn current_switch(&self) -> Option<&Switch> {
        let (c, r, s) = self.selected_switch;
        self.net.closets.get(c)?.racks.get(r)?.switches.get(s)
    }

    pub fn current_closet(&self) -> Option<&IdfCloset> {
        let (c, _, _) = self.selected_switch;
        self.net.closets.get(c)
    }

    pub fn current_rack(&self) -> Option<&Rack> {
        let (c, r, _) = self.selected_switch;
        self.net.closets.get(c)?.racks.get(r)
    }

    pub fn visible_tree_nodes(&self) -> Vec<TreeNode> {
        let mut nodes = Vec::new();

        for (c_idx, closet) in self.net.closets.iter().enumerate() {
            // Apply closet filter if in IssuesOnly mode and no switch has issues
            if self.status_filter == DeviceStatusFilter::IssuesOnly
                && closet.status() == DeviceStatus::Online
            {
                continue;
            }

            nodes.push(TreeNode::Closet(c_idx));

            if self.expanded_closets.get(c_idx).copied().unwrap_or(false) {
                for (r_idx, rack) in closet.racks.iter().enumerate() {
                    let has_matching_switch =
                        rack.switches.iter().any(|sw| match self.status_filter {
                            DeviceStatusFilter::All => true,
                            DeviceStatusFilter::OnlineOnly => sw.status == DeviceStatus::Online,
                            DeviceStatusFilter::IssuesOnly => sw.status != DeviceStatus::Online,
                        });

                    if !has_matching_switch && self.status_filter != DeviceStatusFilter::All {
                        continue;
                    }

                    nodes.push(TreeNode::Rack(c_idx, r_idx));

                    let rack_expanded = self
                        .expanded_racks
                        .get(c_idx)
                        .and_then(|racks| racks.get(r_idx))
                        .copied()
                        .unwrap_or(false);

                    if rack_expanded {
                        for (s_idx, sw) in rack.switches.iter().enumerate() {
                            let matches_filter = match self.status_filter {
                                DeviceStatusFilter::All => true,
                                DeviceStatusFilter::OnlineOnly => sw.status == DeviceStatus::Online,
                                DeviceStatusFilter::IssuesOnly => sw.status != DeviceStatus::Online,
                            };

                            if matches_filter {
                                nodes.push(TreeNode::Switch(c_idx, r_idx, s_idx));
                            }
                        }
                    }
                }
            }
        }

        nodes
    }

    pub fn filtered_interfaces(&self) -> Vec<Interface> {
        let sw = match self.current_switch() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut list: Vec<Interface> = sw
            .interfaces
            .iter()
            .filter(|i| match self.interface_filter {
                InterfaceFilter::All => true,
                InterfaceFilter::UpOnly => i.oper_status == OperStatus::Up,
                InterfaceFilter::DownOnly => i.oper_status == OperStatus::Down,
                InterfaceFilter::ErrDisabledOnly => i.oper_status == OperStatus::ErrDisabled,
                InterfaceFilter::PoeOnly => i.poe_enabled && i.poe_watts > 0.0,
            })
            .cloned()
            .collect();

        list.sort_by(|a, b| {
            let ord = match self.interface_sort {
                InterfaceSortKey::PortName => a.name.cmp(&b.name),
                InterfaceSortKey::Status => a.oper_status.as_str().cmp(b.oper_status.as_str()),
                InterfaceSortKey::Vlan => a.vlan.cmp(&b.vlan),
                InterfaceSortKey::Speed => a.speed.cmp(&b.speed),
                InterfaceSortKey::Poe => a
                    .poe_watts
                    .partial_cmp(&b.poe_watts)
                    .unwrap_or(std::cmp::Ordering::Equal),
                InterfaceSortKey::Errors => {
                    (a.errors_rx + a.errors_tx).cmp(&(b.errors_rx + b.errors_tx))
                }
                InterfaceSortKey::Traffic => a.rx_bytes.cmp(&b.rx_bytes),
            };
            if self.interface_sort_asc {
                ord
            } else {
                ord.reverse()
            }
        });

        list
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // 1. Error modal handles dismiss
        if self.error_modal.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    self.error_modal = None;
                }
                _ => {}
            }
            return;
        }

        // 2. Help modal handles dismiss
        if self.help_modal_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                    self.help_modal_open = false;
                }
                _ => {}
            }
            return;
        }

        // 3. Port detail modal handles dismiss
        if self.port_detail_modal_open {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    self.port_detail_modal_open = false;
                }
                _ => {}
            }
            return;
        }

        // 4. Global Search modal handles input
        if self.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.search_active = false;
                    self.search_query.clear();
                    self.search_results.clear();
                }
                KeyCode::Enter => {
                    self.apply_search_selection();
                }
                KeyCode::Down | KeyCode::Char('j')
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::CONTROL =>
                {
                    if !self.search_results.is_empty() {
                        self.search_cursor = (self.search_cursor + 1) % self.search_results.len();
                    }
                }
                KeyCode::Up | KeyCode::Char('k')
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::CONTROL =>
                {
                    if !self.search_results.is_empty() {
                        if self.search_cursor == 0 {
                            self.search_cursor = self.search_results.len() - 1;
                        } else {
                            self.search_cursor -= 1;
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.update_search_results();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.update_search_results();
                }
                _ => {}
            }
            return;
        }

        // 5. Global key bindings
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.help_modal_open = true;
                return;
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                self.search_results.clear();
                self.search_cursor = 0;
                return;
            }
            KeyCode::Char('e') => {
                // Trigger simulated error to demonstrate error handling modal!
                self.trigger_simulated_error();
                return;
            }
            KeyCode::Char('r') => {
                // Reload / refresh deterministic data
                self.net = crate::mock::generate_mock_infrastructure();
                self.notification = Some(Notification {
                    message: "Deterministic mock topology reloaded successfully.".to_string(),
                    level: NotificationLevel::Success,
                });
                return;
            }
            KeyCode::Tab => {
                let next = (self.active_tab as usize + 1) % Tab::ALL.len();
                self.active_tab = Tab::ALL[next];
                return;
            }
            KeyCode::BackTab => {
                let prev = if (self.active_tab as usize) == 0 {
                    Tab::ALL.len() - 1
                } else {
                    self.active_tab as usize - 1
                };
                self.active_tab = Tab::ALL[prev];
                return;
            }
            KeyCode::Char('1') => {
                self.active_tab = Tab::Dashboard;
                return;
            }
            KeyCode::Char('2') => {
                self.active_tab = Tab::Inventory;
                return;
            }
            KeyCode::Char('3') => {
                self.active_tab = Tab::SwitchDetail;
                return;
            }
            KeyCode::Char('4') => {
                self.active_tab = Tab::Interfaces;
                return;
            }
            KeyCode::Char('5') => {
                self.active_tab = Tab::Alerts;
                return;
            }
            _ => {}
        }

        // 6. Tab-specific key bindings
        match self.active_tab {
            Tab::Dashboard => self.handle_dashboard_key(key),
            Tab::Inventory => self.handle_inventory_key(key),
            Tab::SwitchDetail => self.handle_switch_detail_key(key),
            Tab::Interfaces => self.handle_interfaces_key(key),
            Tab::Alerts => self.handle_alerts_key(key),
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.net.alerts.is_empty() {
                    self.dashboard_alert_cursor =
                        (self.dashboard_alert_cursor + 1).min(self.net.alerts.len() - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.dashboard_alert_cursor > 0 {
                    self.dashboard_alert_cursor -= 1;
                }
            }
            KeyCode::Enter => {
                // Jump to inventory
                self.active_tab = Tab::Inventory;
            }
            _ => {}
        }
    }

    fn handle_inventory_key(&mut self, key: KeyEvent) {
        let nodes = self.visible_tree_nodes();
        if nodes.is_empty() {
            return;
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.tree_cursor + 1 < nodes.len() {
                    self.tree_cursor += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.tree_cursor > 0 {
                    self.tree_cursor -= 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(node) = nodes.get(self.tree_cursor) {
                    match *node {
                        TreeNode::Closet(c) => {
                            if let Some(exp) = self.expanded_closets.get_mut(c) {
                                *exp = !*exp;
                            }
                        }
                        TreeNode::Rack(c, r) => {
                            if let Some(exp) = self
                                .expanded_racks
                                .get_mut(c)
                                .and_then(|racks| racks.get_mut(r))
                            {
                                *exp = !*exp;
                            }
                        }
                        TreeNode::Switch(c, r, s) => {
                            self.selected_switch = (c, r, s);
                            self.selected_interface_idx = 0;
                            self.notification = Some(Notification {
                                message: format!(
                                    "Selected switch: {}",
                                    self.current_switch()
                                        .map(|s| s.hostname.as_str())
                                        .unwrap_or("")
                                ),
                                level: NotificationLevel::Info,
                            });
                        }
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(node) = nodes.get(self.tree_cursor) {
                    match *node {
                        TreeNode::Closet(c) => {
                            if let Some(exp) = self.expanded_closets.get_mut(c) {
                                *exp = true;
                            }
                        }
                        TreeNode::Rack(c, r) => {
                            if let Some(exp) = self
                                .expanded_racks
                                .get_mut(c)
                                .and_then(|racks| racks.get_mut(r))
                            {
                                *exp = true;
                            }
                        }
                        TreeNode::Switch(c, r, s) => {
                            self.selected_switch = (c, r, s);
                            self.active_tab = Tab::SwitchDetail;
                        }
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(node) = nodes.get(self.tree_cursor) {
                    match *node {
                        TreeNode::Closet(c) => {
                            if let Some(exp) = self.expanded_closets.get_mut(c) {
                                *exp = false;
                            }
                        }
                        TreeNode::Rack(c, r) => {
                            if let Some(exp) = self
                                .expanded_racks
                                .get_mut(c)
                                .and_then(|racks| racks.get_mut(r))
                            {
                                *exp = false;
                            }
                        }
                        TreeNode::Switch(c, r, _) => {
                            // Find parent rack
                            for (idx, n) in nodes.iter().enumerate() {
                                if *n == TreeNode::Rack(c, r) {
                                    self.tree_cursor = idx;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('f') => {
                self.status_filter = self.status_filter.cycle();
                self.tree_cursor = 0;
                self.notification = Some(Notification {
                    message: format!("Device filter changed: {}", self.status_filter.as_str()),
                    level: NotificationLevel::Info,
                });
            }
            KeyCode::Char('i') => {
                self.active_tab = Tab::Interfaces;
            }
            KeyCode::Char('d') => {
                self.active_tab = Tab::SwitchDetail;
            }
            _ => {}
        }
    }

    fn handle_switch_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('i') | KeyCode::Enter => {
                self.active_tab = Tab::Interfaces;
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                self.active_tab = Tab::Inventory;
            }
            KeyCode::Char('p') => {
                // Ping test simulation
                if let Some(sw) = self.current_switch() {
                    let ip = sw.ip.clone();
                    let host = sw.hostname.clone();
                    self.notification = Some(Notification {
                        message: format!(
                            "Mock Ping to {host} ({ip}): 5/5 packets received, RTT min/avg/max = 1.1/1.8/2.4 ms"
                        ),
                        level: NotificationLevel::Success,
                    });
                }
            }
            KeyCode::Char('s') | KeyCode::Char('u') => {
                self.sync_selected_switch();
            }
            _ => {}
        }
    }

    fn handle_interfaces_key(&mut self, key: KeyEvent) {
        let ports = self.filtered_interfaces();
        let port_count = ports.len();

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if port_count > 0 && self.selected_interface_idx + 1 < port_count {
                    self.selected_interface_idx += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_interface_idx > 0 {
                    self.selected_interface_idx -= 1;
                }
            }
            KeyCode::PageDown => {
                if port_count > 0 {
                    self.selected_interface_idx =
                        (self.selected_interface_idx + 10).min(port_count - 1);
                }
            }
            KeyCode::PageUp => {
                if self.selected_interface_idx >= 10 {
                    self.selected_interface_idx -= 10;
                } else {
                    self.selected_interface_idx = 0;
                }
            }
            KeyCode::Home => {
                self.selected_interface_idx = 0;
            }
            KeyCode::End => {
                if port_count > 0 {
                    self.selected_interface_idx = port_count - 1;
                }
            }
            KeyCode::Enter => {
                if port_count > 0 {
                    self.port_detail_modal_open = true;
                }
            }
            KeyCode::Char('s') => {
                self.interface_sort = self.interface_sort.cycle();
                self.selected_interface_idx = 0;
                self.notification = Some(Notification {
                    message: format!(
                        "Sorted by: {} ({})",
                        self.interface_sort.as_str(),
                        if self.interface_sort_asc {
                            "Asc"
                        } else {
                            "Desc"
                        }
                    ),
                    level: NotificationLevel::Info,
                });
            }
            KeyCode::Char('S') => {
                self.interface_sort_asc = !self.interface_sort_asc;
                self.notification = Some(Notification {
                    message: format!(
                        "Sort order: {}",
                        if self.interface_sort_asc {
                            "Ascending"
                        } else {
                            "Descending"
                        }
                    ),
                    level: NotificationLevel::Info,
                });
            }
            KeyCode::Char('f') => {
                self.interface_filter = self.interface_filter.cycle();
                self.selected_interface_idx = 0;
                self.notification = Some(Notification {
                    message: format!("Interface filter: {}", self.interface_filter.as_str()),
                    level: NotificationLevel::Info,
                });
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                self.active_tab = Tab::SwitchDetail;
            }
            _ => {}
        }
    }

    fn handle_alerts_key(&mut self, key: KeyEvent) {
        let alert_count = self.net.alerts.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if alert_count > 0 && self.dashboard_alert_cursor + 1 < alert_count {
                    self.dashboard_alert_cursor += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.dashboard_alert_cursor > 0 {
                    self.dashboard_alert_cursor -= 1;
                }
            }
            KeyCode::Esc => {
                self.active_tab = Tab::Dashboard;
            }
            _ => {}
        }
    }

    fn update_search_results(&mut self) {
        self.search_results.clear();
        self.search_cursor = 0;
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            return;
        }

        for (c_idx, closet) in self.net.closets.iter().enumerate() {
            if (closet.name.to_lowercase().contains(&query)
                || closet.building.to_lowercase().contains(&query))
                && let (Some(r_idx), Some(s_idx)) = (
                    closet.racks.first().map(|_| 0),
                    closet
                        .racks
                        .first()
                        .and_then(|r| r.switches.first().map(|_| 0)),
                )
            {
                self.search_results.push(SearchResult {
                    title: format!("Closet: {}", closet.name),
                    subtitle: format!("Location: {} | Floor: {}", closet.building, closet.floor),
                    closet_idx: c_idx,
                    rack_idx: r_idx,
                    switch_idx: s_idx,
                    interface_idx: None,
                });
            }

            for (r_idx, rack) in closet.racks.iter().enumerate() {
                for (s_idx, sw) in rack.switches.iter().enumerate() {
                    let switch_matches = sw.hostname.to_lowercase().contains(&query)
                        || sw.ip.contains(&query)
                        || sw.mac.to_lowercase().contains(&query)
                        || sw.serial.to_lowercase().contains(&query)
                        || sw.model.to_lowercase().contains(&query)
                        || sw.vendor.to_lowercase().contains(&query);

                    if switch_matches {
                        self.search_results.push(SearchResult {
                            title: format!("Switch: {} ({})", sw.hostname, sw.model),
                            subtitle: format!(
                                "IP: {} | Serial: {} | Closet: {}",
                                sw.ip, sw.serial, closet.name
                            ),
                            closet_idx: c_idx,
                            rack_idx: r_idx,
                            switch_idx: s_idx,
                            interface_idx: None,
                        });
                    }

                    for (i_idx, iface) in sw.interfaces.iter().enumerate() {
                        let iface_matches = iface.name.to_lowercase().contains(&query)
                            || iface.description.to_lowercase().contains(&query)
                            || iface.vlan_name.to_lowercase().contains(&query);

                        if iface_matches {
                            self.search_results.push(SearchResult {
                                title: format!(
                                    "Port: {}/{} [{}]",
                                    sw.hostname,
                                    iface.name,
                                    iface.oper_status.as_str()
                                ),
                                subtitle: format!(
                                    "Desc: {} | VLAN: {} ({})",
                                    iface.description, iface.vlan, iface.vlan_name
                                ),
                                closet_idx: c_idx,
                                rack_idx: r_idx,
                                switch_idx: s_idx,
                                interface_idx: Some(i_idx),
                            });
                        }
                    }
                }
            }
        }
    }

    fn apply_search_selection(&mut self) {
        if let Some(res) = self.search_results.get(self.search_cursor) {
            self.selected_switch = (res.closet_idx, res.rack_idx, res.switch_idx);
            if let Some(iface_idx) = res.interface_idx {
                self.selected_interface_idx = iface_idx;
                self.active_tab = Tab::Interfaces;
            } else {
                self.active_tab = Tab::SwitchDetail;
            }
            self.search_active = false;
            self.search_query.clear();
            self.search_results.clear();
            self.notification = Some(Notification {
                message: "Jumped to search target.".to_string(),
                level: NotificationLevel::Success,
            });
        }
    }

    pub fn trigger_simulated_error(&mut self) {
        self.error_modal = Some(
            "SIMULATED SYSTEM ERROR [ERR_COMM_TIMEOUT_504]:
            Failed to query downstream telemetry probe: Operation timed out after 3000ms.
            Error handling subsystem caught this cleanly. Press Esc or Enter to dismiss."
                .to_string(),
        );
        self.notification = Some(Notification {
            message: "Simulated error triggered for demonstration.".to_string(),
            level: NotificationLevel::Error,
        });
    }

    pub fn sync_selected_switch(&mut self) {
        let (c, r, s) = self.selected_switch;
        let is_cisco = self
            .net
            .closets
            .get(c)
            .and_then(|closet| closet.racks.get(r))
            .and_then(|rack| rack.switches.get(s))
            .map(|sw| sw.vendor.eq_ignore_ascii_case("Cisco"))
            .unwrap_or(false);

        if !is_cisco {
            let vendor = self
                .current_switch()
                .map(|sw| sw.vendor.as_str())
                .unwrap_or("Unknown");
            self.notification = Some(Notification {
                message: format!(
                    "Device vendor '{vendor}' is not Cisco IOS-XE. Milestone 2 provides read-only Cisco integration."
                ),
                level: NotificationLevel::Warning,
            });
            return;
        }

        let (target, host) = {
            let sw = match self.current_switch() {
                Some(sw) => sw,
                None => return,
            };
            let creds = crate::provider::DeviceCredentials::with_password(
                "admin",
                crate::provider::CredentialRef::from_secret("CiscoSecret123"),
            );
            let policy = crate::provider::HostKeyPolicy::AcceptNew {
                known_hosts: Vec::new(),
            };
            let target = crate::provider::DeviceTarget::new(
                sw.id.clone(),
                sw.hostname.clone(),
                sw.ip.clone(),
                creds,
                policy,
            );
            (target, sw.hostname.clone())
        };

        if let Some(sw) = self
            .net
            .closets
            .get_mut(c)
            .and_then(|cl| cl.racks.get_mut(r))
            .and_then(|rk| rk.switches.get_mut(s))
        {
            match self.cisco_provider.sync_switch(&target, sw) {
                Ok(()) => {
                    self.notification = Some(Notification {
                        message: format!(
                            "Successfully synced read-only telemetry for {host} via SSH ({} ports, CPU {}%, Temp {}°C)",
                            sw.interfaces.len(),
                            sw.cpu_usage,
                            sw.temp_celsius
                        ),
                        level: NotificationLevel::Success,
                    });
                }
                Err(e) => {
                    let sanitized = crate::provider::LogSanitizer::sanitize_error(&e);
                    self.notification = Some(Notification {
                        message: format!("Failed to sync {host}: {sanitized}"),
                        level: NotificationLevel::Error,
                    });
                }
            }
        }
    }
}
