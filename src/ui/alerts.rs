use crate::{app::App, model::AlertSeverity};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary bar
            Constraint::Min(10),   // Alerts Table
        ])
        .split(area);

    let crit_count = app
        .net
        .alerts
        .iter()
        .filter(|a| a.severity == AlertSeverity::Critical)
        .count();
    let warn_count = app
        .net
        .alerts
        .iter()
        .filter(|a| a.severity == AlertSeverity::Warning)
        .count();
    let info_count = app
        .net
        .alerts
        .iter()
        .filter(|a| a.severity == AlertSeverity::Info)
        .count();

    let sum_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let sum_line = Line::from(vec![
        Span::styled(
            " System Alert Telemetry: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {crit_count} Critical "),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!(" {warn_count} Warning "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!(" {info_count} Info "),
            Style::default().fg(Color::Blue),
        ),
        Span::raw(" | Press [r] to refresh / reload events"),
    ]);
    f.render_widget(Paragraph::new(sum_line).block(sum_block), chunks[0]);

    let alert_count = app.net.alerts.len();
    let title = format!(
        " Syslog & Telemetry Events ({}) [↑/↓ Navigate] ",
        alert_count
    );

    let table_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let header_row = Row::new(vec![
        "ID",
        "Severity",
        "Timestamp",
        "Source / Device",
        "Message",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .net
        .alerts
        .iter()
        .enumerate()
        .map(|(idx, a)| {
            let (sev_str, sev_style) = match a.severity {
                AlertSeverity::Critical => (
                    "[CRITICAL]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                AlertSeverity::Warning => (
                    "[WARNING]",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                AlertSeverity::Info => ("[INFO]", Style::default().fg(Color::Blue)),
            };

            let is_selected = idx == app.dashboard_alert_cursor;
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 45, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                format!("#{}", a.id),
                sev_str.to_string(),
                a.timestamp.clone(),
                a.source.clone(),
                a.message.clone(),
            ])
            .style(row_style)
            .cells([
                ratatui::widgets::Cell::from(format!("#{}", a.id)),
                ratatui::widgets::Cell::from(sev_str).style(sev_style),
                ratatui::widgets::Cell::from(a.timestamp.clone())
                    .style(Style::default().fg(Color::DarkGray)),
                ratatui::widgets::Cell::from(a.source.clone())
                    .style(Style::default().fg(Color::Cyan)),
                ratatui::widgets::Cell::from(a.message.clone())
                    .style(Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(12),
        Constraint::Length(22),
        Constraint::Length(26),
        Constraint::Min(40),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(table_block);
    f.render_widget(table, chunks[1]);
}
