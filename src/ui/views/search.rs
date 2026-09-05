use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;

pub fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let hint = if app.is_searching {
        " (press Enter or Esc to finish)"
    } else {
        " (press / to edit, Esc to clear)"
    };

    let search_line = Line::from(vec![
        Span::styled(" / Search: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        if app.is_searching {
            Span::styled("█", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
        Span::styled(
            hint,
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
