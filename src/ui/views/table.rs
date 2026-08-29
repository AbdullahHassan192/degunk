use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::core::git::GitStatus;
use crate::core::size::format_bytes;
use crate::ui::app::App;
use crate::ui::theme::Theme;

pub fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let filtered_items = app.get_filtered_artifacts();

    let header_cells = [
        Cell::from(Span::styled(" SEL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("PROJECT", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("TYPE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("FOLDER", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("SIZE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("INACTIVITY", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("GIT STATUS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("LOCKFILE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
    ];

    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(25, 30, 42)))
        .height(1);

    let rows: Vec<Row> = filtered_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected_in_tui = app.selected_table_index == idx;

            // Selection box
            let checkmark = if item.is_deleted {
                Span::styled(" [DEL]", Style::default().fg(Color::DarkGray))
            } else if item.is_selected {
                Span::styled(" [✔] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" [ ] ", Style::default().fg(Color::DarkGray))
            };

            // Project name
            let project_span = if item.is_deleted {
                Span::styled(
                    &item.project_name,
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                Span::styled(
                    &item.project_name,
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )
            };

            // Ecosystem badge
            let eco_style = Style::default()
                .fg(Color::Black)
                .bg(Theme::ecosystem_color(item.ecosystem))
                .add_modifier(Modifier::BOLD);
            let eco_span = Span::styled(format!(" {} ", item.ecosystem.badge()), eco_style);

            // Target folder
            let folder_span = Span::styled(&item.folder_name, Style::default().fg(Color::LightCyan));

            // Size
            let size_span = if item.is_deleted {
                Span::styled("cleaned", Style::default().fg(Color::DarkGray))
            } else if item.size_calculated {
                Span::styled(
                    format_bytes(item.size_bytes),
                    Style::default()
                        .fg(Theme::size_color(item.size_bytes))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("calc...", Style::default().fg(Color::DarkGray))
            };

            // Inactivity
            let inactivity_span = if item.days_inactive == 0 {
                Span::styled("Active today", Style::default().fg(Color::Green))
            } else if item.days_inactive < 30 {
                Span::styled(format!("{}d ago", item.days_inactive), Style::default().fg(Color::LightGreen))
            } else if item.days_inactive < 90 {
                Span::styled(format!("{}d ago", item.days_inactive), Style::default().fg(Color::Yellow))
            } else {
                Span::styled(format!("{}d ago", item.days_inactive), Style::default().fg(Color::Red))
            };

            // Git Status
            let git_span = match &item.activity {
                Some(act) => match act.git_status {
                    GitStatus::Clean => Span::styled("Clean", Style::default().fg(Color::Green)),
                    GitStatus::Dirty(n) => Span::styled(
                        format!("Dirty ({})", n),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    GitStatus::Unpushed(n) => Span::styled(
                        format!("Ahead ({})", n),
                        Style::default().fg(Color::LightYellow),
                    ),
                    GitStatus::DirtyAndUnpushed { changed, unpushed } => Span::styled(
                        format!("Dirty:{}/Ah:{}", changed, unpushed),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    GitStatus::NotGit => Span::styled("No Git", Style::default().fg(Color::DarkGray)),
                },
                None => Span::styled("-", Style::default().fg(Color::DarkGray)),
            };

            // Lockfile
            let lockfile_span = if item.has_lockfile {
                Span::styled("✔ Locked", Style::default().fg(Color::Green))
            } else {
                Span::styled("⚠ Missing", Style::default().fg(Color::Yellow))
            };

            let row_style = if is_selected_in_tui {
                Style::default().bg(Color::Rgb(36, 48, 70))
            } else if idx % 2 == 0 {
                Style::default().bg(Color::Rgb(18, 22, 30))
            } else {
                Style::default().bg(Color::Rgb(14, 17, 24))
            };

            Row::new(vec![
                Cell::from(Line::from(checkmark)),
                Cell::from(Line::from(project_span)),
                Cell::from(Line::from(eco_span)),
                Cell::from(Line::from(folder_span)),
                Cell::from(Line::from(size_span)),
                Cell::from(Line::from(inactivity_span)),
                Cell::from(Line::from(git_span)),
                Cell::from(Line::from(lockfile_span)),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(6),          // Checkbox
        Constraint::Percentage(22),     // Project
        Constraint::Length(10),         // Type Badge
        Constraint::Length(18),         // Target Folder
        Constraint::Length(12),         // Size
        Constraint::Length(14),         // Inactivity
        Constraint::Length(16),         // Git Status
        Constraint::Length(12),         // Lockfile
    ];

    let sort_label = format!(" Artifact Targets (Sort: {:?}) ", app.sort_mode);

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    sort_label,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(45, 60, 90))
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    if !filtered_items.is_empty() {
        state.select(Some(app.selected_table_index));
    }

    f.render_stateful_widget(table, area, &mut state);
}
