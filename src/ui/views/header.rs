use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::core::size::format_bytes;
use crate::ui::app::App;

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Title & Version
            Constraint::Percentage(35), // Scan Status & Progress
            Constraint::Percentage(40), // Space Stats
        ])
        .split(area);

    // Left: Title
    let title_line = Line::from(vec![
        Span::styled(" ✦ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(
            "BLACK HOLE",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" v0.1.0 ", Style::default().fg(Color::DarkGray)),
    ]);
    let title_widget = Paragraph::new(title_line)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(title_widget, chunks[0]);

    // Center: Scan Status
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_frames[(app.spinner_tick / 2) % spinner_frames.len()];

    let status_line = if app.is_scanning {
        Line::from(vec![
            Span::styled(
                format!(" {} ", spinner),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Scanning... ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("({} dirs inspected)", app.scanned_dirs_count),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ✔ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("Scan complete ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("({} targets found)", app.artifacts.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };
    let status_widget = Paragraph::new(status_line)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(status_widget, chunks[1]);

    // Right: Space Statistics
    let (selected_count, selected_bytes) = app.get_selected_stats();
    let total_bytes = app.get_total_bytes();

    let stats_line = Line::from(vec![
        Span::styled(" Total: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_bytes(total_bytes),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ Reclaim: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_bytes(selected_bytes),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} selected)", selected_count),
            Style::default().fg(if selected_count > 0 { Color::Cyan } else { Color::DarkGray }),
        ),
    ]);
    let stats_widget = Paragraph::new(stats_line)
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(stats_widget, chunks[2]);
}
