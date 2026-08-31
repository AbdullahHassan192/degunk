use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::core::git::GitStatus;
use crate::core::size::format_bytes;
use crate::ui::app::{App, GroupSelectionState, TableItem};
use crate::ui::theme::Theme;

pub fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_table_items();

    let header_cells = [
        Cell::from(Span::styled(" SEL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("PROJECT / TARGET", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
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

    let rows: Vec<Row> = visible_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected_in_tui = app.selected_table_index == idx;

            match item {
                TableItem::GroupHeader {
                    display_name,
                    primary_ecosystem,
                    artifact_count,
                    total_bytes,
                    days_inactive,
                    activity,
                    all_locked,
                    is_expanded,
                    selection_state,
                    all_deleted,
                    ..
                } => {
                    // Selection checkmark
                    let checkmark = match selection_state {
                        GroupSelectionState::AllDeleted => {
                            Span::styled(" [DEL]", Style::default().fg(Color::DarkGray))
                        }
                        GroupSelectionState::All => Span::styled(
                            " [✔] ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        GroupSelectionState::Partial => Span::styled(
                            " [-] ",
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        GroupSelectionState::None => {
                            Span::styled(" [ ] ", Style::default().fg(Color::DarkGray))
                        }
                    };

                    // Group name with expand glyph
                    let glyph = if *is_expanded { "▼ " } else { "▶ " };
                    let count_suffix = if *artifact_count > 1 {
                        format!(" ({} targets)", artifact_count)
                    } else {
                        String::new()
                    };

                    let project_spans = if *all_deleted {
                        vec![
                            Span::styled(glyph, Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                display_name,
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::CROSSED_OUT),
                            ),
                            Span::styled(count_suffix, Style::default().fg(Color::DarkGray)),
                        ]
                    } else {
                        vec![
                            Span::styled(
                                glyph,
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                display_name,
                                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                count_suffix,
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]
                    };

                    // Ecosystem badge
                    let eco_style = Style::default()
                        .fg(Color::Black)
                        .bg(Theme::ecosystem_color(*primary_ecosystem))
                        .add_modifier(Modifier::BOLD);
                    let eco_span = Span::styled(
                        format!(" {} ", primary_ecosystem.badge()),
                        eco_style,
                    );

                    // Folder summary
                    let folder_text = if *artifact_count > 1 {
                        format!("({} artifacts)", artifact_count)
                    } else {
                        "artifact".to_string()
                    };
                    let folder_span = Span::styled(
                        folder_text,
                        Style::default().fg(Color::DarkGray),
                    );

                    // Total Size
                    let size_span = if *all_deleted {
                        Span::styled("cleaned", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::styled(
                            format_bytes(*total_bytes),
                            Style::default()
                                .fg(Theme::size_color(*total_bytes))
                                .add_modifier(Modifier::BOLD),
                        )
                    };

                    // Inactivity
                    let inactivity_span = if *days_inactive == 0 {
                        Span::styled("Active today", Style::default().fg(Color::Green))
                    } else if *days_inactive < 30 {
                        Span::styled(
                            format!("{}d ago", days_inactive),
                            Style::default().fg(Color::LightGreen),
                        )
                    } else if *days_inactive < 90 {
                        Span::styled(
                            format!("{}d ago", days_inactive),
                            Style::default().fg(Color::Yellow),
                        )
                    } else {
                        Span::styled(
                            format!("{}d ago", days_inactive),
                            Style::default().fg(Color::Red),
                        )
                    };

                    // Git Status
                    let git_span = match activity {
                        Some(act) => match act.git_status {
                            GitStatus::Clean => {
                                Span::styled("Clean", Style::default().fg(Color::Green))
                            }
                            GitStatus::Dirty(n) => Span::styled(
                                format!("Dirty ({})", n),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            GitStatus::Unpushed(n) => Span::styled(
                                format!("Ahead ({})", n),
                                Style::default().fg(Color::LightYellow),
                            ),
                            GitStatus::DirtyAndUnpushed { changed, unpushed } => Span::styled(
                                format!("Dirty:{}/Ah:{}", changed, unpushed),
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ),
                            GitStatus::NotGit => {
                                Span::styled("No Git", Style::default().fg(Color::DarkGray))
                            }
                        },
                        None => Span::styled("-", Style::default().fg(Color::DarkGray)),
                    };

                    // Lockfile
                    let lockfile_span = if *all_locked {
                        Span::styled("✔ Locked", Style::default().fg(Color::Green))
                    } else {
                        Span::styled("⚠ Missing", Style::default().fg(Color::Yellow))
                    };

                    let row_style = if is_selected_in_tui {
                        Style::default().bg(Color::Rgb(40, 56, 85))
                    } else {
                        Style::default().bg(Color::Rgb(24, 30, 42))
                    };

                    Row::new(vec![
                        Cell::from(Line::from(checkmark)),
                        Cell::from(Line::from(project_spans)),
                        Cell::from(Line::from(eco_span)),
                        Cell::from(Line::from(folder_span)),
                        Cell::from(Line::from(size_span)),
                        Cell::from(Line::from(inactivity_span)),
                        Cell::from(Line::from(git_span)),
                        Cell::from(Line::from(lockfile_span)),
                    ])
                    .style(row_style)
                    .height(1)
                }

                TableItem::ChildArtifact {
                    display_label,
                    folder_name,
                    ecosystem,
                    size_bytes,
                    size_calculated,
                    has_lockfile,
                    is_selected,
                    is_deleted,
                    is_last,
                    ..
                } => {
                    // Checkmark indented
                    let checkmark = if *is_deleted {
                        Span::styled("   [DEL]", Style::default().fg(Color::DarkGray))
                    } else if *is_selected {
                        Span::styled(
                            "   [✔] ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled("   [ ] ", Style::default().fg(Color::DarkGray))
                    };

                    // Child tree branch
                    let branch_glyph = if *is_last { "  └─ " } else { "  ├─ " };
                    let project_spans = if *is_deleted {
                        vec![
                            Span::styled(branch_glyph, Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                display_label,
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::CROSSED_OUT),
                            ),
                        ]
                    } else {
                        vec![
                            Span::styled(branch_glyph, Style::default().fg(Color::Cyan)),
                            Span::styled(display_label, Style::default().fg(Color::White)),
                        ]
                    };

                    // Ecosystem badge
                    let eco_style = Style::default()
                        .fg(Color::Black)
                        .bg(Theme::ecosystem_color(*ecosystem));
                    let eco_span =
                        Span::styled(format!(" {} ", ecosystem.badge()), eco_style);

                    // Target folder
                    let folder_span = Span::styled(
                        folder_name,
                        Style::default().fg(Color::LightCyan),
                    );

                    // Size
                    let size_span = if *is_deleted {
                        Span::styled("cleaned", Style::default().fg(Color::DarkGray))
                    } else if *size_calculated {
                        Span::styled(
                            format_bytes(*size_bytes),
                            Style::default().fg(Theme::size_color(*size_bytes)),
                        )
                    } else {
                        Span::styled("calc...", Style::default().fg(Color::DarkGray))
                    };

                    // Inactivity / Git / Lockfile for child
                    let inactivity_span =
                        Span::styled("─", Style::default().fg(Color::DarkGray));
                    let git_span = Span::styled("─", Style::default().fg(Color::DarkGray));

                    let lockfile_span = if *has_lockfile {
                        Span::styled("✔ Locked", Style::default().fg(Color::Green))
                    } else {
                        Span::styled("⚠ Missing", Style::default().fg(Color::Yellow))
                    };

                    let row_style = if is_selected_in_tui {
                        Style::default().bg(Color::Rgb(36, 48, 70))
                    } else if idx % 2 == 0 {
                        Style::default().bg(Color::Rgb(16, 20, 28))
                    } else {
                        Style::default().bg(Color::Rgb(13, 16, 22))
                    };

                    Row::new(vec![
                        Cell::from(Line::from(checkmark)),
                        Cell::from(Line::from(project_spans)),
                        Cell::from(Line::from(eco_span)),
                        Cell::from(Line::from(folder_span)),
                        Cell::from(Line::from(size_span)),
                        Cell::from(Line::from(inactivity_span)),
                        Cell::from(Line::from(git_span)),
                        Cell::from(Line::from(lockfile_span)),
                    ])
                    .style(row_style)
                    .height(1)
                }
            }
        })
        .collect();

    let widths = [
        Constraint::Length(8),          // Checkbox (indented for children)
        Constraint::Percentage(24),     // Project / Target
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
    if !visible_items.is_empty() {
        state.select(Some(app.selected_table_index));
    }

    f.render_stateful_widget(table, area, &mut state);
}

