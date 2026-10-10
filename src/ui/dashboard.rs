use crate::{
    app::App,
    model::{AlertSeverity, DeviceStatus},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // KPI summary cards
            Constraint::Min(10),   // Closets and Architecture cards
            Constraint::Length(7), // Recent alerts banner
        ])
        .split(area);

    render_kpis(f, app, chunks[0]);
    render_middle_section(f, app, chunks[1]);
    render_recent_alerts(f, app, chunks[2]);
}

fn render_kpis(f: &mut Frame, app: &App, area: Rect) {
    let kpi_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(area);

    // 1. Closets KPI
    let closet_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" IDF Closets ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let closet_text = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", app.net.total_closets()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Closets"),
        ]),
        Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{} Online",
                app.net
                    .closets
                    .iter()
                    .filter(|c| c.status() == DeviceStatus::Online)
                    .count()
            )),
            Span::styled("  ▲ ", Style::default().fg(Color::Yellow)),
            Span::raw(format!(
                "{} Degraded",
                app.net
                    .closets
                    .iter()
                    .filter(|c| c.status() != DeviceStatus::Online)
                    .count()
            )),
        ]),
    ];
    f.render_widget(
        Paragraph::new(closet_text).block(closet_block),
        kpi_chunks[0],
    );

    // 2. Switches KPI
    let switch_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Network Switches ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let total_switches = app.net.total_switches();
    let degraded_switches = app
        .net
        .closets
        .iter()
        .flat_map(|c| &c.racks)
        .flat_map(|r| &r.switches)
        .filter(|s| s.status != DeviceStatus::Online)
        .count();
    let switch_text = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", total_switches),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Managed Units"),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} Online", total_switches - degraded_switches),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("  {} Attention", degraded_switches),
                Style::default().fg(if degraded_switches > 0 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(switch_text).block(switch_block),
        kpi_chunks[1],
    );

    // 3. Interfaces KPI
    let iface_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Port Capacity ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let total_ifaces = app.net.total_interfaces();
    let active_ifaces = app.net.active_interfaces();
    let err_ifaces = app.net.err_interfaces();
    let down_ifaces = total_ifaces.saturating_sub(active_ifaces + err_ifaces);
    let iface_text = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", total_ifaces),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Total Switch Ports"),
        ]),
        Line::from(vec![
            Span::styled(
                format!("● Up: {} ", active_ifaces),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("○ Down: {} ", down_ifaces),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("✖ ErrDis: {}", err_ifaces),
                Style::default().fg(if err_ifaces > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(iface_text).block(iface_block), kpi_chunks[2]);

    // 4. PoE Consumption Gauge KPI
    let poe_draw = app.net.total_poe_draw();
    let poe_budget = app.net.total_poe_budget();
    let poe_pct = if poe_budget > 0.0 {
        ((poe_draw / poe_budget) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };
    let poe_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Campus PoE Draw ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let poe_gauge = Gauge::default()
        .block(poe_block)
        .gauge_style(Style::default().fg(if poe_pct > 80 {
            Color::Red
        } else if poe_pct > 60 {
            Color::Yellow
        } else {
            Color::Green
        }))
        .percent(poe_pct)
        .label(format!(
            "{:.1}W / {:.0}W ({}%)",
            poe_draw, poe_budget, poe_pct
        ));
    f.render_widget(poe_gauge, kpi_chunks[3]);
}

fn render_middle_section(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    // Left: Closet Status Cards Table
    let closet_table_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" IDF Closet Infrastructure ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let header_row = Row::new(vec![
        "IDF Closet",
        "Building / Floor",
        "Switches",
        "Status",
        "Temp",
        "UPS Health",
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .net
        .closets
        .iter()
        .map(|c| {
            let status_style = match c.status() {
                DeviceStatus::Online => Style::default().fg(Color::Green),
                DeviceStatus::Degraded => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                DeviceStatus::Offline => {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                }
            };
            let temp_style = if c.ambient_temp_celsius > 30.0 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if c.ambient_temp_celsius > 25.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            };

            Row::new(vec![
                c.name.clone(),
                format!("{}, {}", c.building, c.floor),
                format!("{} units", c.total_switches()),
                c.status().as_str().to_string(),
                format!("{:.1}°C", c.ambient_temp_celsius),
                c.ups_status
                    .split('(')
                    .next()
                    .unwrap_or(&c.ups_status)
                    .trim()
                    .to_string(),
            ])
            .style(Style::default().fg(Color::White))
            .cells([
                ratatui::widgets::Cell::from(c.name.clone()),
                ratatui::widgets::Cell::from(format!("{}, {}", c.building, c.floor)),
                ratatui::widgets::Cell::from(format!("{} units", c.total_switches())),
                ratatui::widgets::Cell::from(c.status().as_str()).style(status_style),
                ratatui::widgets::Cell::from(format!("{:.1}°C", c.ambient_temp_celsius))
                    .style(temp_style),
                ratatui::widgets::Cell::from(
                    c.ups_status
                        .split('(')
                        .next()
                        .unwrap_or(&c.ups_status)
                        .trim()
                        .to_string(),
                ),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(26),
        Constraint::Percentage(28),
        Constraint::Percentage(14),
        Constraint::Percentage(12),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(closet_table_block);
    f.render_widget(table, chunks[0]);

    // Right: Device Fleet & Topology Overview
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Hardware vendor breakdown
    let vendor_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Vendor Distribution ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut cisco = 0;
    let mut juniper = 0;
    let mut arista = 0;
    let mut aruba = 0;
    for c in &app.net.closets {
        for r in &c.racks {
            for s in &r.switches {
                match s.vendor.as_str() {
                    "Cisco" => cisco += 1,
                    "Juniper" => juniper += 1,
                    "Arista" => arista += 1,
                    "Aruba" => aruba += 1,
                    _ => {}
                }
            }
        }
    }

    let vendor_lines = vec![
        Line::from(vec![
            Span::styled(
                " Cisco Systems:  ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{cisco} switches  (IOS-XE 17.x)")),
        ]),
        Line::from(vec![
            Span::styled(
                " Juniper Networks:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {juniper} switches  (Junos 21.4R3)")),
        ]),
        Line::from(vec![
            Span::styled(
                " Arista Networks:",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {arista} switches  (EOS 4.28.2F Spine/Leaf)")),
        ]),
        Line::from(vec![
            Span::styled(
                " Aruba / HPE:    ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{aruba} switches  (AOS-CX 10.10)")),
        ]),
    ];
    f.render_widget(
        Paragraph::new(vendor_lines).block(vendor_block),
        right_chunks[0],
    );

    // Quick System Navigation & Shortcuts Hint
    let nav_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Quick Action Shortcuts ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let nav_lines = vec![
        Line::from(vec![
            Span::styled(
                " [2] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Browse Hierarchical IDF Tree (Closets/Racks/Switches)"),
        ]),
        Line::from(vec![
            Span::styled(
                " [4] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Inspect Switch Interfaces & Port Traffic Counters"),
        ]),
        Line::from(vec![
            Span::styled(
                " [/] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Global Search (Hostnames, IPs, MACs, Serials, VLANs)"),
        ]),
        Line::from(vec![
            Span::styled(
                " [?] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Open Comprehensive Keyboard Shortcuts Help Guide"),
        ]),
    ];
    f.render_widget(Paragraph::new(nav_lines).block(nav_block), right_chunks[1]);
}

fn render_recent_alerts(f: &mut Frame, app: &App, area: Rect) {
    let alert_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            " Active Alerts & Syslog Feed ({} Events) - Press [5] for Details ",
            app.net.alerts.len()
        ))
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let lines: Vec<Line> = app
        .net
        .alerts
        .iter()
        .take(4)
        .map(|a| {
            let (sev_str, sev_style) = match a.severity {
                AlertSeverity::Critical => (
                    "[CRIT]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                AlertSeverity::Warning => (
                    "[WARN]",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                AlertSeverity::Info => (
                    "[INFO]",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
            };

            Line::from(vec![
                Span::styled(format!(" {} ", sev_str), sev_style),
                Span::styled(
                    format!("{} ", a.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("[{}] ", a.source), Style::default().fg(Color::Cyan)),
                Span::raw(&a.message),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines).block(alert_block), area);
}
