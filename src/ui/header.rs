use crate::app::{App, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28), // Logo & Title
            Constraint::Min(45),    // Navigation Tabs
            Constraint::Length(26), // Status & Mode
        ])
        .split(area);

    // Title / Brand
    let title_line = Line::from(vec![
        Span::styled(
            "⚡ IDF-NOC ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "SWITCH MGR",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    f.render_widget(Paragraph::new(title_line).block(title_block), chunks[0]);

    // Navigation Tabs
    let tab_titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Navigation [Tab / 1-6] "),
        )
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(20, 35, 60))
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[1]);

    // Status Indicator
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let status_line = Line::from(vec![
        Span::styled("● ", Style::default().fg(Color::Green)),
        Span::styled(
            "MOCK ENGINE ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[v1.0]", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(status_line).block(status_block), chunks[2]);
}
