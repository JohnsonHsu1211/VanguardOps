use crate::{
    app::{App, TreeNode},
    model::{DeviceStatus, FanStatus, PowerSupplyStatus},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_tree_pane(f, app, chunks[0]);
    render_preview_pane(f, app, chunks[1]);
}

fn render_tree_pane(f: &mut Frame, app: &App, area: Rect) {
    let filter_label = format!(" Filter: {} [f] ", app.status_filter.as_str());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Hierarchical IDF Closet Inventory [↑/↓ Navigate, Space/Enter Expand] ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title_bottom(filter_label)
        .title_alignment(ratatui::layout::Alignment::Left);

    let nodes = app.visible_tree_nodes();
    let mut items = Vec::new();

    for (idx, node) in nodes.iter().enumerate() {
        let is_selected = idx == app.tree_cursor;

        let line = match node {
            TreeNode::Closet(c_idx) => {
                let closet = &app.net.closets[*c_idx];
                let is_expanded = app.expanded_closets.get(*c_idx).copied().unwrap_or(false);
                let expand_icon = if is_expanded { "▼ " } else { "▶ " };
                let (status_icon, status_color) = match closet.status() {
                    DeviceStatus::Online => ("●", Color::Green),
                    DeviceStatus::Degraded => ("▲", Color::Yellow),
                    DeviceStatus::Offline => ("✖", Color::Red),
                };

                Line::from(vec![
                    Span::styled(
                        expand_icon,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{status_icon} "), Style::default().fg(status_color)),
                    Span::styled(
                        &closet.name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(
                            " ({} racks, {} sw)",
                            closet.racks.len(),
                            closet.total_switches()
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
            TreeNode::Rack(c_idx, r_idx) => {
                let rack = &app.net.closets[*c_idx].racks[*r_idx];
                let is_expanded = app
                    .expanded_racks
                    .get(*c_idx)
                    .and_then(|r| r.get(*r_idx))
                    .copied()
                    .unwrap_or(false);
                let expand_icon = if is_expanded { "▼ " } else { "▶ " };

                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(expand_icon, Style::default().fg(Color::Cyan)),
                    Span::styled(&rack.name, Style::default().fg(Color::LightCyan)),
                    Span::styled(
                        format!(" ({}U, {} sw)", rack.ru_capacity, rack.switches.len()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
            TreeNode::Switch(c_idx, r_idx, s_idx) => {
                let sw = &app.net.closets[*c_idx].racks[*r_idx].switches[*s_idx];
                let is_active_switch = app.selected_switch == (*c_idx, *r_idx, *s_idx);
                let (status_badge, status_color) = match sw.status {
                    DeviceStatus::Online => ("[OK]", Color::Green),
                    DeviceStatus::Degraded => ("[WARN]", Color::Yellow),
                    DeviceStatus::Offline => ("[DOWN]", Color::Red),
                };

                let marker = if is_active_switch { " ► " } else { "   " };

                Line::from(vec![
                    Span::raw("        "),
                    Span::styled(
                        marker,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{status_badge} "),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&sw.hostname, Style::default().fg(Color::White)),
                    Span::styled(format!(" ({})", sw.ip), Style::default().fg(Color::Gray)),
                ])
            }
        };

        let item_style = if is_selected {
            Style::default()
                .bg(Color::Rgb(30, 45, 75))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        items.push(ListItem::new(line).style(item_style));
    }

    if items.is_empty() {
        let empty_p = Paragraph::new("No devices match current filter. Press [f] to reset filter.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(empty_p, area);
    } else {
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

fn render_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let sw = match app.current_switch() {
        Some(s) => s,
        None => {
            let p = Paragraph::new("No switch selected").block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Device Preview "),
            );
            f.render_widget(p, area);
            return;
        }
    };

    let closet = app
        .current_closet()
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");
    let rack = app
        .current_rack()
        .map(|r| r.name.as_str())
        .unwrap_or("Unknown");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" Switch Inspector: {} ", sw.hostname))
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Hardware spec
            Constraint::Length(5), // Environment gauges
            Constraint::Length(4), // Port stats
            Constraint::Min(4),    // Actions
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    // 1. Hardware spec lines
    let (stat_color, stat_str) = match sw.status {
        DeviceStatus::Online => (Color::Green, "ONLINE / HEALTHY"),
        DeviceStatus::Degraded => (Color::Yellow, "DEGRADED / CHECK ALERTS"),
        DeviceStatus::Offline => (Color::Red, "OFFLINE / UNREACHABLE"),
    };

    let psu1_color = if sw.psu_1 == PowerSupplyStatus::Ok {
        Color::Green
    } else {
        Color::Red
    };
    let psu2_color = match sw.psu_2 {
        PowerSupplyStatus::RedundantOk | PowerSupplyStatus::Ok => Color::Green,
        PowerSupplyStatus::Fault => Color::Red,
        PowerSupplyStatus::NotPresent => Color::DarkGray,
    };
    let fan_color = match sw.fan_status {
        FanStatus::Normal => Color::Green,
        FanStatus::Warning => Color::Yellow,
        FanStatus::Failed => Color::Red,
    };

    let spec_lines = vec![
        Line::from(vec![
            Span::styled("Status:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                stat_str,
                Style::default().fg(stat_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Stack Role: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sw.stack_role.as_str(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Vendor/Model:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} {}", sw.vendor, sw.model),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Rack Unit:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.rack_unit, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Location:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{closet} > {rack}"),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("IP / MAC:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.ip, Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.mac, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Serial / OS:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.serial, Style::default().fg(Color::White)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.os_version, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("System Uptime: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.uptime, Style::default().fg(Color::White)),
            Span::styled("   PSU1/2: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sw.psu_1.as_str(), Style::default().fg(psu1_color)),
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
            Span::styled(sw.psu_2.as_str(), Style::default().fg(psu2_color)),
            Span::styled("  Fans: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sw.fan_status.as_str(), Style::default().fg(fan_color)),
        ]),
    ];
    f.render_widget(Paragraph::new(spec_lines), inner[0]);

    // 2. Gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner[1]);

    let cpu_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" CPU Load "))
        .gauge_style(Style::default().fg(if sw.cpu_usage > 75 {
            Color::Red
        } else {
            Color::Green
        }))
        .percent(sw.cpu_usage as u16)
        .label(format!("{}%", sw.cpu_usage));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Memory "))
        .gauge_style(Style::default().fg(if sw.mem_usage > 85 {
            Color::Red
        } else {
            Color::Cyan
        }))
        .percent(sw.mem_usage as u16)
        .label(format!("{}%", sw.mem_usage));
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let temp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Chassis Temp "),
        )
        .gauge_style(Style::default().fg(if sw.temp_celsius > 50 {
            Color::Red
        } else if sw.temp_celsius > 42 {
            Color::Yellow
        } else {
            Color::Green
        }))
        .percent(((sw.temp_celsius as f32 / 70.0) * 100.0).clamp(0.0, 100.0) as u16)
        .label(format!("{}°C", sw.temp_celsius));
    f.render_widget(temp_gauge, gauge_chunks[2]);

    // 3. Port stats
    let total_ports = sw.interfaces.len();
    let up_ports = sw.active_ports_count();
    let err_ports = sw.err_ports_count();
    let down_ports = total_ports.saturating_sub(up_ports + err_ports);
    let poe_draw = sw.total_poe_draw();

    let port_lines = vec![
        Line::from(vec![
            Span::styled("Port Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("● Up: {up_ports}   "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("○ Down: {down_ports}   "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("✖ ErrDis: {err_ports}   "),
                Style::default()
                    .fg(if err_ports > 0 {
                        Color::Red
                    } else {
                        Color::Green
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Total: {total_ports}"),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("PoE Draw:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1} W / {:.1} W Budget", poe_draw, sw.poe_budget_watts),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(port_lines), inner[2]);

    // 4. Action hints
    let action_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Switch Actions ");
    let action_lines = vec![Line::from(vec![
        Span::styled(
            " [i] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Open Interface Table    "),
        Span::styled(
            " [d] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Switch Detail View    "),
        Span::styled(
            " [p] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Simulate Ping Test"),
    ])];
    f.render_widget(Paragraph::new(action_lines).block(action_box), inner[3]);
}
