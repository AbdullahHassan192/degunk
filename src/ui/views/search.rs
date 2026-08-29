use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;

pub fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let search_line = Line::from(vec![
        Span::styled(" / Search: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
        Span::styled(
            " (press Enter or Esc to finish)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let widget = Paragraph::new(search_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(widget, area);
}
