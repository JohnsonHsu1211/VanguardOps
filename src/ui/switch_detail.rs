use crate::{
    app::App,
    model::{DeviceStatus, FanStatus, PowerSupplyStatus},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let sw = match app.current_switch() {
        Some(s) => s,
        None => {
            let p = Paragraph::new(
                "No switch selected. Press [2] to select a switch from IDF Inventory.",
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Switch Details "),
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Hardware & Network Overview
            Constraint::Length(7),  // Environmental Gauges
            Constraint::Length(7),  // Port Stats & PoE
            Constraint::Min(4),     // Action Shortcuts
        ])
        .split(area);

    // 1. Hardware Overview
    let (stat_color, stat_str) = match sw.status {
        DeviceStatus::Online => (Color::Green, "ONLINE (Operational)"),
        DeviceStatus::Degraded => (Color::Yellow, "DEGRADED (Attention Required)"),
        DeviceStatus::Offline => (Color::Red, "OFFLINE (Unreachable)"),
    };

    let hw_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" System Overview: {} ", sw.hostname))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let hw_lines = vec![
        Line::from(vec![
            Span::styled("Hostname:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &sw.hostname,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Status:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                stat_str,
                Style::default().fg(stat_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Vendor / Model:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} {}", sw.vendor, sw.model),
                Style::default().fg(Color::LightCyan),
            ),
            Span::styled("   Stack Role:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(sw.stack_role.as_str(), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Location:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{closet} > {rack} (RU {})", sw.rack_unit),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Management IP:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.ip, Style::default().fg(Color::Cyan)),
            Span::styled("   Base MAC:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.mac, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Serial Number:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.serial, Style::default().fg(Color::White)),
            Span::styled("   OS Version:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.os_version, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Uptime:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(&sw.uptime, Style::default().fg(Color::Green)),
            Span::styled("   SNMP / SSH:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enabled (v2c/v3, v2-RSA)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(hw_lines).block(hw_block), chunks[0]);

    // 2. Environmental Telemetry
    let env_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // CPU
            Constraint::Percentage(25), // Memory
            Constraint::Percentage(25), // Temp
            Constraint::Percentage(25), // Power & Fans
        ])
        .split(chunks[1]);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" CPU Utilization "),
        )
        .gauge_style(Style::default().fg(if sw.cpu_usage > 75 {
            Color::Red
        } else {
            Color::Green
        }))
        .percent(sw.cpu_usage as u16)
        .label(format!("{}%", sw.cpu_usage));
    f.render_widget(cpu_gauge, env_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Memory Utilization "),
        )
        .gauge_style(Style::default().fg(if sw.mem_usage > 85 {
            Color::Red
        } else {
            Color::Cyan
        }))
        .percent(sw.mem_usage as u16)
        .label(format!("{}%", sw.mem_usage));
    f.render_widget(mem_gauge, env_chunks[1]);

    let temp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Chassis Thermal "),
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
    f.render_widget(temp_gauge, env_chunks[2]);

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

    let power_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Power & Cooling ");
    let power_lines = vec![
        Line::from(vec![
            Span::raw("PSU-1: "),
            Span::styled(
                sw.psu_1.as_str(),
                Style::default().fg(psu1_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("PSU-2: "),
            Span::styled(
                sw.psu_2.as_str(),
                Style::default().fg(psu2_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Fans:  "),
            Span::styled(
                sw.fan_status.as_str(),
                Style::default().fg(fan_color).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(power_lines).block(power_block),
        env_chunks[3],
    );

    // 3. Port Stats & PoE
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let total_ports = sw.interfaces.len();
    let up_ports = sw.active_ports_count();
    let err_ports = sw.err_ports_count();
    let down_ports = total_ports.saturating_sub(up_ports + err_ports);

    let ports_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Port State Distribution ");
    let ports_lines = vec![
        Line::from(vec![
            Span::styled(
                "Connected (Up):      ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{up_ports} ports"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   Disconnected (Down): ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{down_ports} ports"),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Err-Disabled:        ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{err_ports} ports"),
                Style::default()
                    .fg(if err_ports > 0 {
                        Color::Red
                    } else {
                        Color::Green
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   Total Interfaces:    ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{total_ports} ports"),
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(ports_lines).block(ports_box),
        stats_chunks[0],
    );

    let poe_draw = sw.total_poe_draw();
    let poe_budget = sw.poe_budget_watts;
    let poe_pct = if poe_budget > 0.0 {
        ((poe_draw / poe_budget) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let poe_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Power over Ethernet (PoE) Budget "),
        )
        .gauge_style(Style::default().fg(if poe_pct > 80 {
            Color::Red
        } else {
            Color::Cyan
        }))
        .percent(poe_pct)
        .label(format!(
            "{:.1} W used / {:.1} W available ({}%)",
            poe_draw, poe_budget, poe_pct
        ));
    f.render_widget(poe_gauge, stats_chunks[1]);

    // 4. Quick Actions
    let action_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Actions ");
    let action_lines = vec![Line::from(vec![
        Span::styled(
            " [i / 4] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Open Switch Interface Table       "),
        Span::styled(
            " [p] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Run Mock Ping    "),
        Span::styled(
            " [s] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Sync Cisco SSH Telemetry    "),
        Span::styled(
            " [b / 2] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Back to IDF Inventory Tree"),
    ])];
    f.render_widget(Paragraph::new(action_lines).block(action_box), chunks[3]);
}
