use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;

pub fn render_footer(f: &mut Frame, _app: &App, area: Rect) {
    let keybindings = vec![
        Span::styled(" [Tab] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("View  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Space] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Select  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter/e] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Expand  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[E] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Expand All  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[a] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("All  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[s] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Sort  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[/] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Search  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[d] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("Clean  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[p] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Path  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[r] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("Rescan  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[q] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Quit", Style::default().fg(Color::DarkGray)),
    ];

    let widget = Paragraph::new(Line::from(keybindings)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(widget, area);
}
