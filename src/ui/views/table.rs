use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table,
    },
    Frame,
};

use crate::core::size::format_bytes;
use crate::ui::app::{ActiveTab, App, GroupSelectionState, TableItem};
use crate::ui::theme::Theme;

pub fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        ActiveTab::Projects => render_projects_table(f, app, area),
        ActiveTab::GlobalCaches => render_global_cache_table(f, app, area),
    }
}

fn render_projects_table(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_table_items();
    let num_items = visible_items.len();
    if num_items == 0 {
        app.selected_table_index = 0;
        app.table_state.select(None);
    } else {
        if app.selected_table_index >= num_items {
            app.selected_table_index = num_items - 1;
        }
        app.table_state.select(Some(app.selected_table_index));
    }

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
                    is_deleting,
                    ..
                } => {
                    // Selection checkmark
                    let checkmark = if *all_deleted {
                        Span::styled(" [DEL]", Style::default().fg(Color::DarkGray))
                    } else if *is_deleting {
                        Span::styled(" [CLN]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        match selection_state {
                            GroupSelectionState::AllDeleted => {
                                Span::styled(" [DEL]", Style::default().fg(Color::DarkGray))
                            }
                            GroupSelectionState::All => Span::styled(
                                " [✓] ",
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            ),
                            GroupSelectionState::Partial => Span::styled(
                                " [-] ",
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            ),
                            GroupSelectionState::None => {
                                Span::styled(" [ ] ", Style::default().fg(Color::DarkGray))
                            }
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
                    } else if *is_deleting {
                        vec![
                            Span::styled(
                                glyph,
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                display_name,
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                count_suffix,
                                Style::default().fg(Color::DarkGray),
                            ),
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
                    } else if *is_deleting {
                        Span::styled("cleaning...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
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
                    } else if *days_inactive < 365 {
                        Span::styled(
                            format!("{}d ago", days_inactive),
                            Style::default().fg(Color::Rgb(255, 140, 0)),
                        )
                    } else {
                        Span::styled(
                            format!("{:.1}y ago", *days_inactive as f32 / 365.0),
                            Style::default().fg(Color::Red),
                        )
                    };

                    // Git Status
                    let git_span = if let Some(act) = activity {
                        if !act.is_git {
                            Span::styled("─", Style::default().fg(Color::DarkGray))
                        } else {
                            match &act.git_status {
                                crate::core::git::GitStatus::Clean => {
                                    Span::styled("✓ Clean", Style::default().fg(Color::Green))
                                }
                                crate::core::git::GitStatus::Dirty(uncommitted) => Span::styled(
                                    format!("● {} dirty", uncommitted),
                                    Style::default().fg(Color::Red),
                                ),
                                crate::core::git::GitStatus::Unpushed(commits_ahead) => {
                                    Span::styled(
                                        format!("↑ {} unpushed", commits_ahead),
                                        Style::default().fg(Color::Cyan),
                                    )
                                }
                                crate::core::git::GitStatus::DirtyAndUnpushed {
                                    changed,
                                    unpushed,
                                } => Span::styled(
                                    format!("● {} | ↑ {}", changed, unpushed),
                                    Style::default().fg(Color::Red),
                                ),
                                crate::core::git::GitStatus::NotGit => {
                                    Span::styled("─", Style::default().fg(Color::DarkGray))
                                }
                            }
                        }
                    } else {
                        Span::styled("─", Style::default().fg(Color::DarkGray))
                    };

                    // Lockfile status
                    let lockfile_span = if *all_locked {
                        Span::styled("✓ Locked", Style::default().fg(Color::Green))
                    } else {
                        Span::styled("⚠  Missing", Style::default().fg(Color::Yellow))
                    };

                    let mut row = Row::new(vec![
                        Cell::from(checkmark),
                        Cell::from(Line::from(project_spans)),
                        Cell::from(eco_span),
                        Cell::from(folder_span),
                        Cell::from(size_span),
                        Cell::from(inactivity_span),
                        Cell::from(git_span),
                        Cell::from(lockfile_span),
                    ]);

                    if is_selected_in_tui {
                        row = row.style(
                            Style::default()
                                .bg(Color::Rgb(40, 50, 75))
                                .add_modifier(Modifier::BOLD),
                        );
                    } else {
                        row = row.style(Style::default().bg(Color::Rgb(20, 24, 34)));
                    }

                    row
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
                    is_deleting,
                    is_last,
                    ..
                } => {
                    // Checkmark indented
                    let checkmark = if *is_deleted {
                        Span::styled("   [DEL]", Style::default().fg(Color::DarkGray))
                    } else if *is_deleting {
                        Span::styled("   [CLN]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else if *is_selected {
                        Span::styled(
                            "   [✓] ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled("   [ ] ", Style::default().fg(Color::DarkGray))
                    };

                    // Child tree branch
                    let branch_glyph = if *is_last { "  └── " } else { "  ├── " };
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
                    } else if *is_deleting {
                        vec![
                            Span::styled(branch_glyph, Style::default().fg(Color::Yellow)),
                            Span::styled(display_label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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
                    } else if *is_deleting {
                        Span::styled("cleaning...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
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
                        Span::styled("✓ Locked", Style::default().fg(Color::Green))
                    } else {
                        Span::styled("⚠  Missing", Style::default().fg(Color::Yellow))
                    };

                    let mut row = Row::new(vec![
                        Cell::from(checkmark),
                        Cell::from(Line::from(project_spans)),
                        Cell::from(eco_span),
                        Cell::from(folder_span),
                        Cell::from(size_span),
                        Cell::from(inactivity_span),
                        Cell::from(git_span),
                        Cell::from(lockfile_span),
                    ]);

                    if is_selected_in_tui {
                        row = row.style(
                            Style::default()
                                .bg(Color::Rgb(35, 42, 60))
                                .add_modifier(Modifier::BOLD),
                        );
                    } else {
                        row = row.style(Style::default().fg(Color::DarkGray));
                    }

                    row
                }
            }
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Percentage(28),
        Constraint::Length(11),
        Constraint::Length(14),
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Length(16),
        Constraint::Length(12),
    ];

    let pos_indicator = if num_items > 0 {
        format!(" [{}/{}]", app.selected_table_index + 1, num_items)
    } else {
        String::new()
    };

    let title = if let Some(ref eco) = app.allowed_ecosystems {
        let ecos: Vec<_> = eco.iter().map(|e| e.name()).collect();
        format!(" Artifacts [Filtered: {}] (Sort: {:?}){} ", ecos.join(", "), app.sort_mode, pos_indicator)
    } else {
        format!(" Artifacts (Sort: {:?}){} ", app.sort_mode, pos_indicator)
    };

    let table = Table::new(rows, widths)
        .header(header_row)
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(40, 80, 120)))
                .title(Span::styled(
                    title,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
        );

    f.render_stateful_widget(table, area, &mut app.table_state);

    if num_items > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        let scrollbar_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        let mut scrollbar_state = ScrollbarState::new(num_items).position(app.selected_table_index);
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn render_global_cache_table(f: &mut Frame, app: &mut App, area: Rect) {
    let num_caches = app.get_visible_global_caches().len();
    if num_caches == 0 {
        app.selected_cache_index = 0;
        app.cache_table_state.select(None);
    } else {
        if app.selected_cache_index >= num_caches {
            app.selected_cache_index = num_caches - 1;
        }
        app.cache_table_state.select(Some(app.selected_cache_index));
    }

    let header_cells = [
        Cell::from(Span::styled(" SEL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("DEVELOPER TOOL CACHE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("TYPE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("SYSTEM PATH", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("SIZE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("FILES", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("CLEAN HINT / COMMAND", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
    ];

    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(25, 30, 42)))
        .height(1);

    let rows: Vec<Row> = {
        let visible_caches = app.get_visible_global_caches();
        visible_caches
            .into_iter()
            .enumerate()
            .map(|(idx, (_orig_idx, cache))| {
                let is_selected_in_tui = app.selected_cache_index == idx;
                let is_deleting = app.deleting_paths.contains(&cache.path);

                let checkmark = if cache.is_deleted {
                    Span::styled(" [DEL]", Style::default().fg(Color::DarkGray))
                } else if is_deleting {
                    Span::styled(" [CLN]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else if cache.is_selected {
                    Span::styled(
                        " [✓] ",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(" [ ] ", Style::default().fg(Color::DarkGray))
                };

                let name_span = if cache.is_deleted {
                    Span::styled(
                        cache.name.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::CROSSED_OUT),
                    )
                } else if is_deleting {
                    Span::styled(
                        cache.name.clone(),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        cache.name.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    )
                };

                let eco_style = Style::default()
                    .fg(Color::Black)
                    .bg(Theme::ecosystem_color(cache.ecosystem))
                    .add_modifier(Modifier::BOLD);
                let eco_span = Span::styled(format!(" {} ", cache.ecosystem.badge()), eco_style);

                let path_span = Span::styled(
                    cache.path.display().to_string(),
                    Style::default().fg(if cache.is_deleted { Color::DarkGray } else { Color::LightCyan }),
                );

                let size_span = if cache.is_deleted {
                    Span::styled("cleaned", Style::default().fg(Color::DarkGray))
                } else if is_deleting {
                    Span::styled("cleaning...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else if cache.size_calculated {
                    Span::styled(
                        format_bytes(cache.size_bytes),
                        Style::default()
                            .fg(Theme::size_color(cache.size_bytes))
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("calc...", Style::default().fg(Color::DarkGray))
                };

                let files_span = if cache.is_deleted {
                    Span::styled("─", Style::default().fg(Color::DarkGray))
                } else if is_deleting {
                    Span::styled("cleaning", Style::default().fg(Color::Yellow))
                } else if cache.size_calculated {
                    Span::styled(
                        format!("{} files", cache.file_count),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::styled("─", Style::default().fg(Color::DarkGray))
                };

                let hint_span = Span::styled(
                    cache.clean_hint.clone(),
                    Style::default().fg(if cache.is_deleted { Color::DarkGray } else { Color::Yellow }),
                );

                let mut row = Row::new(vec![
                    Cell::from(checkmark),
                    Cell::from(name_span),
                    Cell::from(eco_span),
                    Cell::from(path_span),
                    Cell::from(size_span),
                    Cell::from(files_span),
                    Cell::from(hint_span),
                ]);

                if is_selected_in_tui {
                    row = row.style(
                        Style::default()
                            .bg(Color::Rgb(40, 50, 75))
                            .add_modifier(Modifier::BOLD),
                    );
                }

                row
            })
            .collect()
    };

    let widths = [
        Constraint::Length(8),
        Constraint::Length(25),
        Constraint::Length(11),
        Constraint::Percentage(35),
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Percentage(25),
    ];

    let pos_indicator = if num_caches > 0 {
        format!(" [{}/{}]", app.selected_cache_index + 1, num_caches)
    } else {
        String::new()
    };

    let total_bytes: u64 = app.global_caches.iter().filter(|c| !c.is_deleted).map(|c| c.size_bytes).sum();
    let title = format!(
        " Global Developer Tool Caches (Sort: {:?}) ({}){} ",
        app.sort_mode,
        format_bytes(total_bytes),
        pos_indicator
    );

    let table = Table::new(rows, widths)
        .header(header_row)
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(40, 80, 120)))
                .title(Span::styled(
                    title,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
        );

    f.render_stateful_widget(table, area, &mut app.cache_table_state);

    if num_caches > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        let scrollbar_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        let mut scrollbar_state = ScrollbarState::new(num_caches).position(app.selected_cache_index);
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
