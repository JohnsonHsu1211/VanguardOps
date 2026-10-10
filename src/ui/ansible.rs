use crate::ansible::status::JobState;
use crate::app::App;
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
            Constraint::Length(3), // Top Summary / Policy bar
            Constraint::Min(10),   // Split: Job Audit List (Left) + Job Inspector (Right)
        ])
        .split(area);

    render_policy_bar(f, app, chunks[0]);
    render_main_split(f, app, chunks[1]);
}

fn render_policy_bar(f: &mut Frame, app: &App, area: Rect) {
    let write_enabled = app.ansible.approval_gate.policy.write_operations_enabled;
    let write_badge = if write_enabled {
        Span::styled(
            " WRITE ENABLED ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " WRITE LOCKED (Safe Default) ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let entries = app.ansible.audit_trail.all_entries();
    let total_jobs = entries.len();
    let completed_jobs = entries
        .iter()
        .filter(|e| e.state == JobState::Completed)
        .count();
    let awaiting_jobs = entries
        .iter()
        .filter(|e| e.state == JobState::AwaitingApproval)
        .count();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let line = Line::from(vec![
        Span::styled(
            " Ansible Automation Adapter: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        write_badge,
        Span::raw(" | "),
        Span::styled(
            format!("Jobs: {total_jobs} total "),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("({completed_jobs} completed, "),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!("{awaiting_jobs} pending approval) "),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | [d] Run Pre-Flight Dry-Run  [a] Approve Change"),
    ]);

    f.render_widget(Paragraph::new(line).block(block), area);
}

fn render_main_split(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(48), // Left: Jobs / Audit History Table
            Constraint::Percentage(52), // Right: Job Inspector & Per-Device Status
        ])
        .split(area);

    let entries = app.ansible.audit_trail.all_entries();
    render_audit_table(f, app, &entries, chunks[0]);
    render_job_inspector(f, app, &entries, chunks[1]);
}

fn render_audit_table(
    f: &mut Frame,
    app: &App,
    entries: &[crate::ansible::AuditEntry],
    area: Rect,
) {
    let title = format!(
        " Automation Audit History ({} Records) [↑/↓ Navigate] ",
        entries.len()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let header_row = Row::new(vec!["Job ID", "Playbook", "Mode", "State", "Duration"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(idx, e)| {
            let is_selected = idx == app.selected_audit_idx;
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 45, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let state_style = match e.state {
                JobState::Completed => Style::default().fg(Color::Green),
                JobState::Running => Style::default().fg(Color::Cyan),
                JobState::AwaitingApproval => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                JobState::Failed | JobState::TimedOut => {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                }
                JobState::Cancelled => Style::default().fg(Color::DarkGray),
                JobState::Pending | JobState::Approved => Style::default().fg(Color::White),
            };

            let mode_str = if e.is_dry_run { "Dry-Run" } else { "Live" };
            let dur_str = format!("{:.1}s", e.duration.as_secs_f32());

            Row::new(vec![
                e.job_id.clone(),
                e.playbook_id.clone(),
                mode_str.to_string(),
                e.state.as_str().to_string(),
                dur_str.clone(),
            ])
            .style(row_style)
            .cells([
                ratatui::widgets::Cell::from(e.job_id.clone())
                    .style(Style::default().fg(Color::White)),
                ratatui::widgets::Cell::from(e.playbook_id.clone()),
                ratatui::widgets::Cell::from(mode_str).style(if e.is_dry_run {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }),
                ratatui::widgets::Cell::from(e.state.as_str()).style(state_style),
                ratatui::widgets::Cell::from(dur_str).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(16),
        Constraint::Percentage(34),
        Constraint::Length(10),
        Constraint::Length(18),
        Constraint::Min(8),
    ];

    let table = Table::new(rows, widths).header(header_row).block(block);
    f.render_widget(table, area);
}

fn render_job_inspector(
    f: &mut Frame,
    app: &App,
    entries: &[crate::ansible::AuditEntry],
    area: Rect,
) {
    let entry = entries.get(app.selected_audit_idx);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Job Inspector & Per-Device Telemetry ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let e = match entry {
        Some(entry) => entry,
        None => {
            let p =
                Paragraph::new("No automation jobs recorded. Press [d] to run pre-flight dry-run.")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(block);
            f.render_widget(p, area);
            return;
        }
    };

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Job metadata & approval
            Constraint::Min(6),    // Per-device status table
            Constraint::Length(5), // Post-change verification
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    // 1. Job Metadata
    let meta_lines = vec![
        Line::from(vec![
            Span::styled("Job ID:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &e.job_id,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Timestamp: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&e.timestamp, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Playbook:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&e.playbook_name, Style::default().fg(Color::Yellow)),
            Span::styled("   Type: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                e.playbook_type.as_str(),
                Style::default().fg(Color::LightCyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Operator:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&e.operator, Style::default().fg(Color::White)),
            Span::styled("   Approval: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:?}", e.approval_status),
                Style::default().fg(if e.approval_status.is_approved() {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Command:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&e.sanitized_command, Style::default().fg(Color::DarkGray)),
        ]),
    ];
    f.render_widget(Paragraph::new(meta_lines), inner[0]);

    render_job_tables(f, e, inner[1], inner[2]);

    fn render_job_tables(
        f: &mut Frame,
        e: &crate::ansible::AuditEntry,
        table_area: Rect,
        ver_area: Rect,
    ) {
        use crate::ansible::status::DeviceExecutionState;
        use crate::ansible::verification::VerificationStatus;

        // 2. Per-Device Status Table
        let dev_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Per-Device Execution Status (PLAY RECAP) ");

        let dev_header = Row::new(vec![
            "Target Switch",
            "Status",
            "ok",
            "changed",
            "unreach",
            "failed",
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(0);

        let dev_rows: Vec<Row> = e
            .per_device_status
            .values()
            .map(|d| {
                let (badge, style) = match d.state {
                    DeviceExecutionState::Ok => ("OK", Style::default().fg(Color::Green)),
                    DeviceExecutionState::Changed => {
                        ("CHANGED", Style::default().fg(Color::Yellow))
                    }
                    DeviceExecutionState::Failed => (
                        "FAILED",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    DeviceExecutionState::Unreachable => (
                        "UNREACHABLE",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    DeviceExecutionState::Running => ("RUNNING", Style::default().fg(Color::Cyan)),
                    DeviceExecutionState::Pending => {
                        ("PENDING", Style::default().fg(Color::DarkGray))
                    }
                };

                Row::new(vec![
                    d.device.clone(),
                    badge.to_string(),
                    format!("{}", d.ok_count),
                    format!("{}", d.changed_count),
                    format!("{}", d.unreachable_count),
                    format!("{}", d.failed_count),
                ])
                .cells([
                    ratatui::widgets::Cell::from(d.device.clone())
                        .style(Style::default().fg(Color::White)),
                    ratatui::widgets::Cell::from(badge).style(style),
                    ratatui::widgets::Cell::from(format!("{}", d.ok_count)),
                    ratatui::widgets::Cell::from(format!("{}", d.changed_count)),
                    ratatui::widgets::Cell::from(format!("{}", d.unreachable_count)),
                    ratatui::widgets::Cell::from(format!("{}", d.failed_count)),
                ])
            })
            .collect();

        let dev_widths = [
            Constraint::Percentage(40),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
        ];

        let dev_table = Table::new(dev_rows, dev_widths)
            .header(dev_header)
            .block(dev_block);
        f.render_widget(dev_table, table_area);

        // 3. Post-Change Verification Report
        let ver_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Post-Change Health Verification ");

        let ver_content = if let Some(ref ver) = e.verification {
            let (v_color, v_text) = match ver.status {
                VerificationStatus::Passed => (Color::Green, "PASSED"),
                VerificationStatus::Warning => (Color::Yellow, "WARNING"),
                VerificationStatus::Failed => (Color::Red, "FAILED"),
                VerificationStatus::Skipped => (Color::DarkGray, "SKIPPED"),
            };

            vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        v_text,
                        Style::default().fg(v_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  —  {}", ver.summary),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    format!(
                        "Checks: {} passed / {} total",
                        ver.checks.iter().filter(|c| c.passed).count(),
                        ver.checks.len()
                    ),
                    Style::default().fg(Color::Gray),
                )]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No verification report recorded for this job.",
                Style::default().fg(Color::DarkGray),
            ))]
        };

        f.render_widget(Paragraph::new(ver_content).block(ver_block), ver_area);
    }
}
