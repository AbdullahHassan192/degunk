use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::core::deleter::DeleteMode;
use crate::core::git::GitStatus;
use crate::core::size::format_bytes;
use crate::ui::app::{ActiveTab, App, DeletionState, PathPickerState, PathPickerTarget};

pub fn render_modal(f: &mut Frame, app: &App) {
    if let Some(ref picker) = app.path_picker {
        render_path_picker_modal(f, app, picker);
        return;
    }

    if app.deletion_state == DeletionState::Idle {
        return;
    }

    let area = match &app.deletion_state {
        DeletionState::Done { errors, .. } if *errors > 0 => centered_rect(72, 65, f.area()),
        DeletionState::Confirming => centered_rect(70, 55, f.area()),
        _ => centered_rect(65, 50, f.area()),
    };
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
            error_details,
            log_path,
        } => {
            render_done_modal(
                f,
                area,
                *freed_bytes,
                *errors,
                *mode,
                *cancelled,
                error_details,
                log_path.as_deref(),
            );
        }
        DeletionState::Idle => {}
    }
}

fn render_confirm_modal(f: &mut Frame, app: &App, area: Rect) {
    let (selected_count, selected_bytes) = app.get_selected_stats();
    let is_global_caches = app.active_tab == ActiveTab::GlobalCaches;

    let mut lines = Vec::new();

    lines.push(Line::from(""));
    if is_global_caches {
        let item_desc = if selected_count == 1 {
            "1 global cache".to_string()
        } else {
            format!("{} global caches", selected_count)
        };
        lines.push(Line::from(vec![
            Span::styled("  You are about to clean ", Style::default().fg(Color::White)),
            Span::styled(
                item_desc,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to reclaim ", Style::default().fg(Color::White)),
            Span::styled(
                format_bytes(selected_bytes),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" of disk space.", Style::default().fg(Color::White)),
        ]));
    } else {
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
    }
    lines.push(Line::from(""));

    // Warnings section
    if is_global_caches {
        lines.push(Line::from(Span::styled(
            "  ── Global Cache Warning ────────────────────────────",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(vec![
            Span::styled("  ⚠  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                "These are system-wide shared tool caches, not project build directories.",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled(
                "Global tool caches will require re-downloading by their respective package managers.",
                Style::default().fg(Color::Rgb(255, 200, 100)),
            ),
        ]));
        lines.push(Line::from(""));
    } else {
        let selected_items: Vec<_> = app.artifacts.iter().filter(|a| a.is_selected && !a.is_deleted).collect();

        let dirty_count = selected_items
            .iter()
            .filter(|a| matches!(a.activity.as_ref().map(|act| &act.git_status), Some(GitStatus::Dirty(_)) | Some(GitStatus::DirtyAndUnpushed { .. })))
            .count();

        let missing_lock_count = selected_items
            .iter()
            .filter(|a| !a.has_lockfile)
            .count();

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

    let title = if is_global_caches {
        " Clean Confirmation (Global Caches) "
    } else {
        " Clean Confirmation "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            title,
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
                let pos = spinner_tick % cycle;
                let mut bar = String::with_capacity(bar_width);
                for i in 0..bar_width {
                    if i + block_size >= pos && i < pos {
                        bar.push('━');
                    } else {
                        bar.push('─');
                    }
                }
                (
                    "Moving to Trash",
                    bar,
                    "Moving item to OS Trash...".to_string(),
                )
            } else {
                // Multi-target progress percentage
                let pct = (completed_targets as f64 / total_targets as f64).clamp(0.0, 1.0);
                let filled = (pct * bar_width as f64).round() as usize;
                let empty = bar_width.saturating_sub(filled);
                let bar = format!("{}{}", "━".repeat(filled), "─".repeat(empty));
                let status = format!(
                    "Moving {}/{} targets... ({})",
                    completed_targets,
                    total_targets,
                    format_bytes(freed_bytes)
                );
                ("Moving to Trash", bar, status)
            }
        }
        DeleteMode::Permanent => {
            let pct = if total_bytes > 0 {
                (freed_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0)
            } else if total_targets > 0 {
                (completed_targets as f64 / total_targets as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let filled = (pct * bar_width as f64).round() as usize;
            let empty = bar_width.saturating_sub(filled);
            let bar = format!("{}{}", "━".repeat(filled), "─".repeat(empty));
            let status = format!(
                "Deleting {}/{} targets... ({} / {})",
                completed_targets,
                total_targets,
                format_bytes(freed_bytes),
                format_bytes(total_bytes)
            );
            ("Deleting Permanently", bar, status)
        }
    };

    let mut lines = Vec::new();
    lines.push(Line::from(""));

    let action_color = match mode {
        DeleteMode::Trash => Color::LightCyan,
        DeleteMode::Permanent => Color::Red,
    };

    // Header with spinner
    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", spinner), Style::default().fg(action_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}...", action_title), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // Progress bar
    lines.push(Line::from(vec![
        Span::styled("  [", Style::default().fg(Color::DarkGray)),
        Span::styled(bar_str, Style::default().fg(action_color).add_modifier(Modifier::BOLD)),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled(status_line, Style::default().fg(Color::LightGreen)),
    ]));
    lines.push(Line::from(""));

    // Current item being deleted
    let short_path = if current_path.len() > 50 {
        format!("...{}", &current_path[current_path.len() - 47..])
    } else {
        current_path.to_string()
    };

    lines.push(Line::from(vec![
        Span::styled("  Current: ", Style::default().fg(Color::DarkGray)),
        Span::styled(short_path, Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    // Footer with cancel instruction
    if is_cancelling {
        lines.push(Line::from(Span::styled(
            "  Cancelling deletion... please wait",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("to abort deletion", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(action_color).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            format!(" Cleaning in Progress ({}/{}) ", current_target, total_targets),
            Style::default().fg(action_color).add_modifier(Modifier::BOLD),
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
    error_details: &[String],
    log_path: Option<&Path>,
) {
    let mut lines = Vec::new();
    lines.push(Line::from(""));

    let mode_str = match mode {
        DeleteMode::Trash => "moved to Trash / Recycle Bin",
        DeleteMode::Permanent => "permanently deleted",
    };

    if cancelled {
        lines.push(Line::from(vec![
            Span::styled("  ⚠  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Deletion Cancelled by User", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
    } else if errors > 0 {
        lines.push(Line::from(vec![
            Span::styled("  ⚠  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Clean completed with warnings / errors", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ✓  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("Clean completed successfully!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  Freed: ", Style::default().fg(Color::White)),
        Span::styled(
            format_bytes(freed_bytes),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" disk space ({})", mode_str), Style::default().fg(Color::White)),
    ]));

    if errors > 0 {
        lines.push(Line::from(vec![
            Span::styled("  Errors: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{} item(s) could not be removed", errors),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]));

        if !error_details.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Failed Targets (Files in use or permission denied):",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));

            let max_display_errors = 4;
            for (i, err) in error_details.iter().take(max_display_errors).enumerate() {
                let err_str = if err.len() > 64 {
                    format!("...{}", &err[err.len() - 61..])
                } else {
                    err.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("    {}. ", i + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(err_str, Style::default().fg(Color::LightYellow)),
                ]));
            }

            if error_details.len() > max_display_errors {
                lines.push(Line::from(Span::styled(
                    format!("    ... and {} more error(s)", error_details.len() - max_display_errors),
                    Style::default().fg(Color::DarkGray),
                )));
            }

            if let Some(log) = log_path {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  Detailed Log: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(log.display().to_string(), Style::default().fg(Color::White)),
                ]));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("or ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("to return to table", Style::default().fg(Color::DarkGray)),
    ]));

    let border_color = if cancelled {
        Color::Yellow
    } else if errors > 0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let title_text = if cancelled {
        " Cleanup Interrupted "
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
        "  (Supports %USERPROFILE%, ~, relative paths, or absolute drive paths)",
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
    } else if !picker.custom_input.trim().is_empty() {
        if let Some(resolved) = crate::core::paths::resolve_user_path(&picker.custom_input) {
            lines.push(Line::from(vec![
                Span::styled("  ✓  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("Resolves to: {}", resolved.display()),
                    Style::default().fg(Color::Green),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ℹ  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Waiting for valid directory path...",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
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
