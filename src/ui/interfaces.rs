use crate::{app::App, model::OperStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
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
                    .title(" Interfaces "),
            );
            f.render_widget(p, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Control bar: filters & sorting
            Constraint::Min(10),   // Interface Table
        ])
        .split(area);

    // 1. Controls bar
    let ctrl_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let ctrl_line = Line::from(vec![
        Span::styled(
            format!(" Switch: {} ", sw.hostname),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Filter [f]: {} ", app.interface_filter.as_str()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!(
                "Sort [s/S]: {} ({}) ",
                app.interface_sort.as_str(),
                if app.interface_sort_asc {
                    "Asc"
                } else {
                    "Desc"
                }
            ),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Showing {} ports", app.filtered_interfaces().len()),
            Style::default().fg(Color::White),
        ),
    ]);
    f.render_widget(Paragraph::new(ctrl_line).block(ctrl_block), chunks[0]);

    // 2. Interfaces Table
    let ports = app.filtered_interfaces();
    let port_count = ports.len();

    let title = format!(
        " Interface Ports Table (Row {}/{}) [↑/↓ Navigate, Enter Details, s/S Sort, f Filter] ",
        if port_count > 0 {
            app.selected_interface_idx + 1
        } else {
            0
        },
        port_count
    );

    let table_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let header_row = Row::new(vec![
        "Port",
        "Admin",
        "Status",
        "Mode",
        "VLAN",
        "Speed",
        "PoE (W)",
        "Rx MB",
        "Tx MB",
        "Errors",
        "Connected Neighbor / Description",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(0);

    // Visible window for table scrolling
    let visible_rows_count = chunks[1].height.saturating_sub(4) as usize;
    let visible_rows_count = visible_rows_count.max(1);

    let start_idx = if app.selected_interface_idx >= visible_rows_count {
        app.selected_interface_idx - visible_rows_count + 1
    } else {
        0
    };
    let _end_idx = (start_idx + visible_rows_count).min(port_count);

    let rows: Vec<Row> = ports
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(visible_rows_count)
        .map(|(orig_idx, p)| {
            let is_selected = orig_idx == app.selected_interface_idx;

            let (status_str, status_style) = match p.oper_status {
                OperStatus::Up => (
                    "connected",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                OperStatus::Down => ("notconnect", Style::default().fg(Color::DarkGray)),
                OperStatus::ErrDisabled => (
                    "err-disabled",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                OperStatus::Testing => ("testing", Style::default().fg(Color::Yellow)),
            };

            let total_errors = p.errors_rx + p.errors_tx;
            let err_style = if total_errors > 0 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let poe_str = if p.poe_enabled && p.poe_watts > 0.0 {
                format!("{:.1}W", p.poe_watts)
            } else {
                "-".to_string()
            };

            let rx_mb = p.rx_bytes / (1024 * 1024);
            let tx_mb = p.tx_bytes / (1024 * 1024);

            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 45, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                p.name.clone(),
                p.admin_status.as_str().to_string(),
                status_str.to_string(),
                p.mode.as_str().to_string(),
                format!("{} ({})", p.vlan, p.vlan_name),
                p.speed.clone(),
                poe_str,
                format!("{rx_mb}"),
                format!("{tx_mb}"),
                format!("{total_errors}"),
                p.description.clone(),
            ])
            .style(row_style)
            .cells([
                ratatui::widgets::Cell::from(p.name.clone())
                    .style(Style::default().fg(Color::White)),
                ratatui::widgets::Cell::from(p.admin_status.as_str()),
                ratatui::widgets::Cell::from(status_str).style(status_style),
                ratatui::widgets::Cell::from(p.mode.as_str()),
                ratatui::widgets::Cell::from(format!("{} ({})", p.vlan, p.vlan_name)),
                ratatui::widgets::Cell::from(p.speed.clone()),
                ratatui::widgets::Cell::from(if p.poe_enabled && p.poe_watts > 0.0 {
                    format!("{:.1}W", p.poe_watts)
                } else {
                    "-".to_string()
                })
                .style(if p.poe_enabled {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
                ratatui::widgets::Cell::from(format!("{rx_mb}")),
                ratatui::widgets::Cell::from(format!("{tx_mb}")),
                ratatui::widgets::Cell::from(format!("{total_errors}")).style(err_style),
                ratatui::widgets::Cell::from(p.description.clone())
                    .style(Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10), // Port
        Constraint::Length(7),  // Admin
        Constraint::Length(14), // Status
        Constraint::Length(8),  // Mode
        Constraint::Length(16), // VLAN
        Constraint::Length(11), // Speed
        Constraint::Length(9),  // PoE
        Constraint::Length(9),  // Rx MB
        Constraint::Length(9),  // Tx MB
        Constraint::Length(8),  // Errors
        Constraint::Min(25),    // Description
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(table_block);
    f.render_widget(table, chunks[1]);
}
