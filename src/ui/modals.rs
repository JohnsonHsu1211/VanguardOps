use crate::{app::App, model::OperStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_help(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 75, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" Keyboard Shortcuts & System Navigation Guide [Esc/Enter to Close] ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let help_text = vec![
        Line::from(vec![Span::styled(
            "--- GLOBAL SHORTCUTS ---",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  1 - 6        : ", Style::default().fg(Color::Yellow)),
            Span::raw(
                "Direct switch between main tabs (Dashboard, Inventory, Details, Interfaces, Alerts, Ansible)",
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tab / S-Tab  : ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle forward / backward between tabs"),
        ]),
        Line::from(vec![
            Span::styled("  /            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Open Global Search (search switch hostnames, IPs, MACs, ports, VLANs)"),
        ]),
        Line::from(vec![
            Span::styled("  r            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Reset / Reload deterministic mock network topology"),
        ]),
        Line::from(vec![
            Span::styled("  e            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Trigger simulated error (demonstrates error handling modal & recovery)"),
        ]),
        Line::from(vec![
            Span::styled("  ?            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this Keyboard Shortcuts Help modal"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc      : ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit application / Close active modal or search"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- HIERARCHICAL IDF INVENTORY TREE ---",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ (j / k): ", Style::default().fg(Color::Yellow)),
            Span::raw("Navigate tree nodes (Closets, Racks, Switches)"),
        ]),
        Line::from(vec![
            Span::styled("  Space / Enter: ", Style::default().fg(Color::Yellow)),
            Span::raw("Expand / Collapse Closet or Rack; Select Switch"),
        ]),
        Line::from(vec![
            Span::styled("  → / ← (l / h): ", Style::default().fg(Color::Yellow)),
            Span::raw("Expand node or jump to Switch Details / Collapse node"),
        ]),
        Line::from(vec![
            Span::styled("  f            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle Device Status Filter (All Devices -> Online Only -> Issues Only)"),
        ]),
        Line::from(vec![
            Span::styled("  i / d        : ", Style::default().fg(Color::Yellow)),
            Span::raw("Direct jump to Interfaces Table / Switch Details for selected switch"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- INTERFACES TABLE ---",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ (j / k): ", Style::default().fg(Color::Yellow)),
            Span::raw("Select port row"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp / PgDn  : ", Style::default().fg(Color::Yellow)),
            Span::raw("Page up / down 10 rows; Home / End jump to start/end"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        : ", Style::default().fg(Color::Yellow)),
            Span::raw("Open detailed Port Diagnostics & Telemetry modal"),
        ]),
        Line::from(vec![
            Span::styled("  s / S        : ", Style::default().fg(Color::Yellow)),
            Span::raw(
                "Cycle Sort column (Port, Status, VLAN, Speed, PoE, Errors, Traffic) / Toggle Asc/Desc",
            ),
        ]),
        Line::from(vec![
            Span::styled("  f            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle Interface Filter (All -> Up Only -> Down Only -> ErrDisabled -> PoE)"),
        ]),
        Line::from(vec![
            Span::styled("  b / Esc      : ", Style::default().fg(Color::Yellow)),
            Span::raw("Return to Switch Details view"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- ANSIBLE & AUDIT WORKFLOW ---",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ (j / k): ", Style::default().fg(Color::Yellow)),
            Span::raw("Navigate historical automation jobs"),
        ]),
        Line::from(vec![
            Span::styled("  d            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Execute pre-flight Dry-Run verification (--check --diff)"),
        ]),
        Line::from(vec![
            Span::styled("  a            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Authorize & approve pending staged change request"),
        ]),
        Line::from(vec![
            Span::styled("  w            : ", Style::default().fg(Color::Yellow)),
            Span::raw("Test safety policy gate (verifies write protection lock)"),
        ]),
    ];

    let p = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup_area);
}

pub fn render_search(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(75, 70, area);
    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(6),    // Results list
            Constraint::Length(2), // Hint footer
        ])
        .split(popup_area);

    // Search query box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" Global Search [Type to search, Esc to Cancel] ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let query_line = Line::from(vec![
        Span::styled(
            "🔍 Search: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.search_query,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(Color::Yellow)), // Blinking cursor look
    ]);
    f.render_widget(Paragraph::new(query_line).block(input_block), chunks[0]);

    // Results list
    let res_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            " Matches ({}) [↑/↓ Navigate, Enter Select] ",
            app.search_results.len()
        ));

    if app.search_results.is_empty() {
        let empty_msg = if app.search_query.is_empty() {
            "Type hostname, IP address, MAC, serial, switch model, closet, or port description..."
        } else {
            "No matching network elements found."
        };
        let p = Paragraph::new(empty_msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(res_block);
        f.render_widget(p, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(idx, res)| {
                let is_selected = idx == app.search_cursor;
                let style = if is_selected {
                    Style::default()
                        .bg(Color::Rgb(30, 45, 75))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let marker = if is_selected { "► " } else { "  " };

                let line = Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        &res.title,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  —  {}", res.subtitle),
                        Style::default().fg(Color::Gray),
                    ),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(res_block);
        f.render_widget(list, chunks[1]);
    }

    // Hint footer
    let hint = Paragraph::new("Press Enter to jump to target switch/port | Esc to close search")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, chunks[2]);
}

pub fn render_port_detail(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(65, 70, area);
    f.render_widget(Clear, popup_area);

    let ports = app.filtered_interfaces();
    let port = match ports.get(app.selected_interface_idx) {
        Some(p) => p,
        None => return,
    };

    let sw = match app.current_switch() {
        Some(s) => s,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(format!(
            " Port Diagnostics: {} on {} [Esc/Enter to Close] ",
            port.name, sw.hostname
        ))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let (status_str, status_style) = match port.oper_status {
        OperStatus::Up => (
            "CONNECTED / OPERATIONAL",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        OperStatus::Down => ("NOT CONNECTED / DOWN", Style::default().fg(Color::DarkGray)),
        OperStatus::ErrDisabled => (
            "ERR-DISABLED (Check violations)",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        OperStatus::Testing => ("TESTING", Style::default().fg(Color::Yellow)),
    };

    let poe_str = if port.poe_enabled {
        format!("Enabled ({:.1} Watts active draw)", port.poe_watts)
    } else {
        "Disabled / Inactive".to_string()
    };

    let mac_str = port
        .mac_learned
        .clone()
        .unwrap_or_else(|| "No dynamic MAC learned".to_string());

    let rx_mb = port.rx_bytes as f64 / (1024.0 * 1024.0);
    let tx_mb = port.tx_bytes as f64 / (1024.0 * 1024.0);

    let lines = vec![
        Line::from(vec![
            Span::styled("Interface Name:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &port.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Admin State: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                port.admin_status.as_str(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Oper Status:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(status_str, status_style),
        ]),
        Line::from(vec![
            Span::styled("Description:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&port.description, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- LAYER 2 / SWITCHING CONFIGURATION ---",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![
            Span::styled("Switchport Mode: ", Style::default().fg(Color::DarkGray)),
            Span::styled(port.mode.as_str(), Style::default().fg(Color::White)),
            Span::styled("   VLAN Assignment: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ({})", port.vlan, port.vlan_name),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Speed / Duplex:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} / {}", port.speed, port.duplex),
                Style::default().fg(Color::White),
            ),
            Span::styled("   MTU Size: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} bytes", port.mtu),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Learned MAC:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(mac_str, Style::default().fg(Color::LightCyan)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- POWER OVER ETHERNET (PoE) ---",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![
            Span::styled("PoE Status:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                poe_str,
                Style::default().fg(if port.poe_enabled {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "--- TRAFFIC TELEMETRY & ERROR COUNTERS ---",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![
            Span::styled("Rx Throughput:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2} MB ({} bytes)", rx_mb, port.rx_bytes),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tx Throughput:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2} MB ({} bytes)", tx_mb, port.tx_bytes),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Rx Errors:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} packets", port.errors_rx),
                Style::default().fg(if port.errors_rx > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
            Span::styled("   Tx Errors: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} packets", port.errors_tx),
                Style::default().fg(if port.errors_tx > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Esc or Enter to dismiss this inspector.",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup_area);
}

pub fn render_error(f: &mut Frame, msg: &str, area: Rect) {
    let popup_area = centered_rect(60, 40, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" System Fault / Error Alert [Esc/Enter to Dismiss] ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ⚠️  ATTENTION REQUIRED:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::raw(format!("  {msg}"))]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Press Esc, Enter, or 'q' to acknowledge and recover.",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup_area);
}
