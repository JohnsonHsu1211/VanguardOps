pub mod alerts;
pub mod dashboard;
pub mod footer;
pub mod header;
pub mod interfaces;
pub mod inventory;
pub mod modals;
pub mod switch_detail;

use crate::app::{App, Tab};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render_app(f: &mut Frame, app: &App) {
    let area = f.area();

    // Responsive layout check: min width 70, min height 18
    if area.width < 70 || area.height < 18 {
        render_compact_fallback(f, app, area);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Content view
            Constraint::Length(3), // Footer & Keybindings
        ])
        .split(area);

    // 1. Header
    header::render(f, app, main_chunks[0]);

    // 2. Active Tab View
    match app.active_tab {
        Tab::Dashboard => dashboard::render(f, app, main_chunks[1]),
        Tab::Inventory => inventory::render(f, app, main_chunks[1]),
        Tab::SwitchDetail => switch_detail::render(f, app, main_chunks[1]),
        Tab::Interfaces => interfaces::render(f, app, main_chunks[1]),
        Tab::Alerts => alerts::render(f, app, main_chunks[1]),
    }

    // 3. Footer
    footer::render(f, app, main_chunks[2]);

    // 4. Overlays & Modals
    if let Some(ref err_msg) = app.error_modal {
        modals::render_error(f, err_msg, area);
    } else if app.help_modal_open {
        modals::render_help(f, area);
    } else if app.search_active {
        modals::render_search(f, app, area);
    } else if app.port_detail_modal_open {
        modals::render_port_detail(f, app, area);
    }
}

fn render_compact_fallback(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" IDF-NOC Switch Manager [Compact Mode] ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));

    let lines = vec![
        Line::from(vec![Span::styled(
            "⚠️ Terminal Dimensions Constrained",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!(
            "Current size: {}x{} | Recommended: 80x24+",
            area.width, area.height
        )),
        Line::from(""),
        Line::from(format!(
            "Total IDFs: {} | Total Switches: {}",
            app.net.total_closets(),
            app.net.total_switches()
        )),
        Line::from(format!(
            "Total Ports: {} (Up: {}, ErrDis: {})",
            app.net.total_interfaces(),
            app.net.active_interfaces(),
            app.net.err_interfaces()
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press 'q' to exit or expand terminal window to view full UI.",
            Style::default().fg(Color::Cyan),
        )]),
    ];

    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}
