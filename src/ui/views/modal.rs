use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::core::git::GitStatus;
use crate::core::size::format_bytes;
use crate::ui::app::{App, DeletionState};

pub fn render_modal(f: &mut Frame, app: &App) {
    if app.deletion_state == DeletionState::Idle {
        return;
    }

    let area = centered_rect(65, 45, f.area());
    f.render_widget(Clear, area); // Clears the background behind the modal

    match app.deletion_state {
        DeletionState::Confirming => {
            render_confirm_modal(f, app, area);
        }
        DeletionState::Deleting => {
            render_progress_modal(f, app, area);
        }
        DeletionState::Done { freed_bytes, errors } => {
            render_done_modal(f, area, freed_bytes, errors);
        }
        DeletionState::Idle => {}
    }
}

fn render_confirm_modal(f: &mut Frame, app: &App, area: Rect) {
    let (selected_count, selected_bytes) = app.get_selected_stats();
    let selected_items: Vec<_> = app.artifacts.iter().filter(|a| a.is_selected && !a.is_deleted).collect();

    let dirty_count = selected_items
        .iter()
        .filter(|a| matches!(a.activity.as_ref().map(|act| &act.git_status), Some(GitStatus::Dirty(_)) | Some(GitStatus::DirtyAndUnpushed { .. })))
        .count();

    let missing_lock_count = selected_items
        .iter()
        .filter(|a| !a.has_lockfile)
        .count();

    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  You are about to clean ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{} targets", selected_count),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to reclaim ", Style::default().fg(Color::White)),
        Span::styled(
            format_bytes(selected_bytes),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" of disk space.", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(""));

    // Warnings section
    if dirty_count > 0 || missing_lock_count > 0 {
        lines.push(Line::from(Span::styled(
            "  ── Safety Warnings ──────────────────────────────────",
            Style::default().fg(Color::Yellow),
        )));

        if dirty_count > 0 {
            lines.push(Line::from(vec![
                Span::styled("  ⚠ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} selected project(s) have uncommitted git changes!", dirty_count),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }

        if missing_lock_count > 0 {
            lines.push(Line::from(vec![
                Span::styled("  ⚠ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} selected project(s) do not have a lockfile.", missing_lock_count),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Choose an action:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("    [T] ", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        Span::styled("Move to Trash / Recycle Bin ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("(Safe, easily recoverable)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("    [P] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("Permanently Delete ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("(Instant direct wipe, unrecoverable)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("or ", Style::default().fg(Color::DarkGray)),
        Span::styled("[c] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("to cancel", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " Clean Confirmation ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_progress_modal(f: &mut Frame, _app: &App, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Cleaning selected artifacts...",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Please wait while files are being removed.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(" Cleaning In Progress ", Style::default().fg(Color::Yellow)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_done_modal(f: &mut Frame, area: Rect, freed_bytes: u64, errors: usize) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✔ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Cleanup Complete!",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Successfully reclaimed ", Style::default().fg(Color::White)),
            Span::styled(
                format_bytes(freed_bytes),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" of disk space.", Style::default().fg(Color::White)),
        ]),
    ];

    if errors > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  ⚠ Encountered {} error(s) during deletion.", errors),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("or ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("to continue", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(" Done ", Style::default().fg(Color::Green)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Helper function to create a centered Rect.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
