use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::core::size::format_bytes;
use crate::ui::app::{ActiveTab, App};

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // Title & Version
            Constraint::Min(44),    // View Tabs
            Constraint::Length(36), // Space Stats & Selection
        ])
        .split(area);

    // Left: Title
    let title_line = Line::from(vec![
        Span::styled(
            " DEGUNK",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" v0.1.0 ", Style::default().fg(Color::DarkGray)),
    ]);
    let title_widget = Paragraph::new(title_line)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(title_widget, chunks[0]);

    // Center: View Tabs ([1] Projects vs [2] Global Caches)
    let proj_total = format_bytes(app.get_total_project_bytes());
    let cache_total = format_bytes(app.get_total_global_cache_bytes());

    let (tab1_style, tab2_style) = match app.active_tab {
        ActiveTab::Projects => (
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Style::default()
                .fg(Color::DarkGray),
        ),
        ActiveTab::GlobalCaches => (
            Style::default()
                .fg(Color::DarkGray),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let tabs_line = Line::from(vec![
        Span::raw(" View: "),
        Span::styled(format!(" [1] Projects ({}) ", proj_total), tab1_style),
        Span::raw(" "),
        Span::styled(format!(" [2] Global Caches ({}) ", cache_total), tab2_style),
    ]);
    let tabs_widget = Paragraph::new(tabs_line)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(tabs_widget, chunks[1]);

    // Right: Space Stats & Selection
    let (_, selected_bytes) = app.get_selected_stats();
    let current_total = app.get_total_bytes();

    let stats_line = Line::from(vec![
        Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_bytes(current_total),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ Reclaim: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_bytes(selected_bytes),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);
    let stats_widget = Paragraph::new(stats_line)
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(stats_widget, chunks[2]);
}
