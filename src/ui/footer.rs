use crate::app::{App, NotificationLevel, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // Notifications or context breadcrumbs
            Constraint::Percentage(45), // Keybindings
        ])
        .split(area);

    // Left: Notification or Active Scope
    let notif_line = if let Some(ref notif) = app.notification {
        let (badge, style) = match notif.level {
            NotificationLevel::Info => ("[INFO]", Style::default().fg(Color::Cyan)),
            NotificationLevel::Success => ("[OK]", Style::default().fg(Color::Green)),
            NotificationLevel::Warning => ("[WARN]", Style::default().fg(Color::Yellow)),
            NotificationLevel::Error => (
                "[ERR]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        };
        Line::from(vec![
            Span::styled(format!(" {badge} "), style),
            Span::raw(&notif.message),
        ])
    } else {
        let sw_name = app
            .current_switch()
            .map(|s| s.hostname.as_str())
            .unwrap_or("None");
        Line::from(vec![
            Span::styled(" Active Switch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                sw_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    f.render_widget(Paragraph::new(notif_line).block(left_block), chunks[0]);

    // Right: Keybinding guide
    let keys_text = match app.active_tab {
        Tab::Dashboard => " [1-5] Tabs  [/] Search  [e] Mock Err  [r] Reset  [?] Help  [q] Quit",
        Tab::Inventory => " [↑/↓] Select  [Enter/Space] Expand  [f] Filter  [i] Ports  [?] Help",
        Tab::SwitchDetail => {
            " [i] Ports  [s] Sync Cisco SSH  [p] Ping  [b] Back  [/] Search  [?] Help"
        }
        Tab::Interfaces => " [↑/↓] Row  [Enter] Detail  [s/S] Sort  [f] Filter  [b] Back  [?] Help",
        Tab::Alerts => " [↑/↓] Scroll  [1-4] Tabs  [/] Search  [r] Refresh  [?] Help",
    };

    let right_line = Line::from(vec![Span::styled(
        keys_text,
        Style::default().fg(Color::LightYellow),
    )]);
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Keybindings ");
    f.render_widget(Paragraph::new(right_line).block(right_block), chunks[1]);
}
