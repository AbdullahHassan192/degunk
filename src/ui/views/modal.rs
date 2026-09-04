use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::sync::atomic::Ordering;

use crate::core::deleter::DeleteMode;
use crate::core::git::GitStatus;
use crate::core::size::format_bytes;
use crate::ui::app::{App, DeletionState, PathPickerState, PathPickerTarget};

pub fn render_modal(f: &mut Frame, app: &App) {
    if let Some(ref picker) = app.path_picker {
        render_path_picker_modal(f, app, picker);
        return;
    }

    if app.deletion_state == DeletionState::Idle {
        return;
    }

    let area = centered_rect(65, 50, f.area());
    f.render_widget(Clear, area); // Clears the background behind the modal

    match &app.deletion_state {
        DeletionState::Confirming => {
            render_confirm_modal(f, app, area);
        }
        DeletionState::Deleting {
            current_target,
            total_targets,
            completed_targets,
            current_path,
            freed_bytes,
            total_bytes,
            mode,
        } => {
            let is_cancelling = app
                .deletion_cancel
                .as_ref()
                .map_or(false, |c| c.load(Ordering::Relaxed));

            render_progress_modal(
                f,
                area,
                app.spinner_tick,
                *current_target,
                *total_targets,
                *completed_targets,
                current_path,
                *freed_bytes,
                *total_bytes,
                *mode,
                is_cancelling,
            );
        }
        DeletionState::Done {
            freed_bytes,
            errors,
            mode,
            cancelled,
        } => {
            render_done_modal(f, area, *freed_bytes, *errors, *mode, *cancelled);
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
                Span::styled("  ⚠  ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} selected project(s) have uncommitted git changes!", dirty_count),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }

        if missing_lock_count > 0 {
            lines.push(Line::from(vec![
                Span::styled("  ⚠  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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

fn render_progress_modal(
    f: &mut Frame,
    area: Rect,
    spinner_tick: usize,
    current_target: usize,
    total_targets: usize,
    completed_targets: usize,
    current_path: &str,
    freed_bytes: u64,
    total_bytes: u64,
    mode: DeleteMode,
    is_cancelling: bool,
) {
    let bar_width = 30usize;
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_frames[(spinner_tick / 2) % spinner_frames.len()];

    let (action_title, bar_str, status_line) = match mode {
        DeleteMode::Trash => {
            if total_targets <= 1 {
                // Indeterminate marquee progress bar for atomic OS move to Recycle Bin
                let block_size = 8usize;
                let cycle = bar_width + block_size;
                let pos = (spinner_tick / 2) % cycle;
                let mut chars = vec!['░'; bar_width];
                for i in 0..block_size {
                    if pos >= i && (pos - i) < bar_width {
                        chars[pos - i] = '█';
                    }
                }
                let bar_display: String = chars.into_iter().collect();
                (
                    "Moving to Trash...".to_string(),
                    format!("[{}] moving...", bar_display),
                    Line::from(vec![
                        Span::styled("  Target size: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{} (moving into Recycle Bin)", format_bytes(total_bytes)),
                            Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                )
            } else {
                let pct = (completed_targets * 100) / total_targets;
                let filled = (pct * bar_width) / 100;
                let empty = bar_width.saturating_sub(filled);
                (
                    format!("Moving target {} of {} to Trash...", current_target, total_targets),
                    format!("[{}{}] {}% ({} of {})", "█".repeat(filled), "░".repeat(empty), pct, completed_targets, total_targets),
                    Line::from(vec![
                        Span::styled("  Moved to Trash: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{} / {}", format_bytes(freed_bytes), format_bytes(total_bytes)),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                )
            }
        }
        DeleteMode::Permanent => {
            let pct = if total_bytes > 0 {
                ((freed_bytes as f64 / total_bytes as f64) * 100.0).min(100.0) as usize
            } else if total_targets > 0 {
                (completed_targets * 100) / total_targets
            } else {
                0
            };

            let filled = (pct * bar_width) / 100;
            let empty = bar_width.saturating_sub(filled);
            (
                format!("Cleaning target {} of {}...", current_target, total_targets),
                format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), pct),
                Line::from(vec![
                    Span::styled("  Reclaimed so far: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if total_bytes > 0 {
                            format!("{} / {}", format_bytes(freed_bytes), format_bytes(total_bytes))
                        } else {
                            format_bytes(freed_bytes)
                        },
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                ]),
            )
        }
    };

    let display_path = if current_path.len() > 45 {
        format!("...{}", &current_path[current_path.len().saturating_sub(42)..])
    } else {
        current_path.to_string()
    };

    let cancel_hint_line = if is_cancelling {
        Line::from(vec![
            Span::styled("  ⚠  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Cancelling deletion... stopping workers", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("or ", Style::default().fg(Color::DarkGray)),
            Span::styled("[c] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("to cancel", Style::default().fg(Color::DarkGray)),
        ])
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(Color::Yellow)),
            Span::styled(
                action_title,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Progress: ", Style::default().fg(Color::DarkGray)),
            Span::styled(bar_str, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        status_line,
        Line::from(""),
        Line::from(vec![
            Span::styled("  Current: ", Style::default().fg(Color::DarkGray)),
            Span::styled(display_path, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        cancel_hint_line,
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_cancelling { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Yellow) })
        .title(Span::styled(
            if is_cancelling { " Cancelling Deletion... " } else { " Cleaning In Progress " },
            Style::default().fg(Color::Yellow),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_done_modal(
    f: &mut Frame,
    area: Rect,
    freed_bytes: u64,
    errors: usize,
    mode: DeleteMode,
    cancelled: bool,
) {
    let mut lines = if cancelled {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⚠   ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(
                    "Deletion Cancelled",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Stopped early by user. Reclaimed ", Style::default().fg(Color::White)),
                Span::styled(
                    format_bytes(freed_bytes),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" before cancellation.", Style::default().fg(Color::White)),
            ]),
        ]
    } else {
        match mode {
            DeleteMode::Trash => vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ✓   ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Moved to Recycle Bin!",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Successfully moved ", Style::default().fg(Color::White)),
                    Span::styled(
                        format_bytes(freed_bytes),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to the Recycle Bin.", Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ℹ   Storage Note: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Files are in your Recycle Bin and remain recoverable.",
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(Span::styled(
                    "    To permanently free up disk space, remember to empty your Recycle Bin.",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            DeleteMode::Permanent => vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ✓   ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
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
            ],
        }
    };

    if errors > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  ⚠  Encountered {} error(s) during deletion.", errors),
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

    let border_color = if cancelled {
        Color::Yellow
    } else if errors > 0 {
        Color::Red
    } else {
        Color::Green
    };

    let title_text = if cancelled {
        " Deletion Cancelled "
    } else if errors > 0 {
        " Cleanup Finished with Errors "
    } else {
        " Done "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, Style::default().fg(border_color)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_path_picker_modal(f: &mut Frame, app: &App, picker: &PathPickerState) {
    let area = centered_rect(74, 65, f.area());
    f.render_widget(Clear, area);

    if picker.is_entering_custom {
        render_custom_path_input(f, app, picker, area);
    } else {
        render_picker_list(f, app, picker, area);
    }
}

fn render_picker_list(f: &mut Frame, app: &App, picker: &PathPickerState, area: Rect) {
    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Select a location to scan for developer dependencies and build caches:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  ───────────────────────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    let inner_height = area.height.saturating_sub(7) as usize;
    let total_items = picker.items.len();
    let scroll_offset = if total_items > inner_height && picker.selected_index >= inner_height {
        picker.selected_index - inner_height + 1
    } else {
        0
    };

    let max_label_len = picker
        .items
        .iter()
        .map(|it| it.label.len())
        .max()
        .unwrap_or(18)
        .max(18);

    for (idx, item) in picker.items.iter().enumerate().skip(scroll_offset).take(inner_height) {
        let is_selected = idx == picker.selected_index;

        let prefix = if is_selected {
            Span::styled("  ▸ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("    ")
        };

        let key_badge = match item.shortcut {
            Some(ch) => {
                let color = if ch == 'C' || ch == 'G' {
                    Color::LightCyan
                } else {
                    Color::Cyan
                };
                Span::styled(format!("[{}] ", ch), Style::default().fg(color).add_modifier(Modifier::BOLD))
            }
            None => Span::styled("[-] ", Style::default().fg(Color::DarkGray)),
        };

        let label_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let path_style = if is_selected {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let display_desc = match item.target {
            PathPickerTarget::GlobalCaches => {
                let cache_bytes = app.get_total_global_cache_bytes();
                if cache_bytes > 0 {
                    format!("{} central caches (Cargo, npm, Ollama...)", format_bytes(cache_bytes))
                } else {
                    item.path_display.clone()
                }
            }
            _ => item.path_display.clone(),
        };

        let padded_label = format!("{:<width$}", item.label, width = max_label_len);

        let mut spans = vec![
            prefix,
            key_badge,
            Span::styled(padded_label, label_style),
        ];

        if !display_desc.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(format!("({})", display_desc), path_style));
        }

        let line = Line::from(spans);
        let styled_line = if is_selected {
            line.patch_style(Style::default().bg(Color::Rgb(30, 42, 65)))
        } else {
            line
        };

        lines.push(styled_line);
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ───────────────────────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::styled("  [↑/↓] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Navigate   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Scan   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[1-9] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Quick Pick   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[C] ", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        Span::styled("Custom Path   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[G] ", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        Span::styled("Global Caches   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc/q] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Quit", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " Select Target Directory to Scan ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_custom_path_input(f: &mut Frame, app: &App, picker: &PathPickerState, area: Rect) {
    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter or paste directory path to scan:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  (Supports Windows drives like D:\\projects, relative paths, or ~/code)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let cursor = if (app.spinner_tick / 4) % 2 == 0 { "█" } else { " " };
    let input_line = Line::from(vec![
        Span::styled("  Path: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(&picker.custom_input, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
    ]);
    lines.push(input_line);
    lines.push(Line::from(""));

    if let Some(ref err) = picker.custom_error {
        lines.push(Line::from(vec![
            Span::styled("  ⚠  ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(err, Style::default().fg(Color::Red)),
        ]));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ───────────────────────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::styled("  [Enter] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Start Scan        ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Back to List", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " Enter Custom Directory Path ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

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
