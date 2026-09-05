use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io::{stdout, Result};
use std::time::Duration;

use crate::core::deleter::DeleteMode;
use crate::ui::app::{App, DeletionState};
use crate::ui::views::{
    footer::render_footer,
    header::render_header,
    intro::render_intro,
    modal::render_modal,
    search::render_search_bar,
    table::render_table,
};

pub fn run_tui(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook to restore terminal
    let original_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_panic(panic_info);
    }));

    let result = event_loop(&mut terminal, &mut app);

    // Teardown
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    while !app.should_quit {
        app.tick();

        if let Some(ref intro) = app.intro_state {
            if intro.is_finished() {
                app.intro_state = None;
            }
        }

        terminal.draw(|f| {
            if let Some(ref intro) = app.intro_state {
                render_intro(f, intro);
                return;
            }

            let total_area = f.area();

            let show_search = app.is_searching || !app.search_query.is_empty();
            let constraints = if show_search {
                vec![
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Search bar
                    Constraint::Min(5),    // Table
                    Constraint::Length(3), // Footer
                ]
            } else {
                vec![
                    Constraint::Length(3), // Header
                    Constraint::Min(5),    // Table
                    Constraint::Length(3), // Footer
                ]
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(total_area);

            if show_search {
                render_header(f, app, chunks[0]);
                render_search_bar(f, app, chunks[1]);
                render_table(f, app, chunks[2]);
                render_footer(f, app, chunks[3]);
            } else {
                render_header(f, app, chunks[0]);
                render_table(f, app, chunks[1]);
                render_footer(f, app, chunks[2]);
            }

            // Render popup modal if active
            render_modal(f, app);
        })?;

        let poll_duration = if app.intro_state.is_some() {
            Duration::from_millis(25)
        } else {
            Duration::from_millis(50)
        };

        // Poll events
        if event::poll(poll_duration)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        if app.intro_state.is_some() {
                            app.intro_state = None;
                        } else {
                            handle_key(app, key.code, key.modifiers);
                        }
                    }
                }
                Event::Paste(text) => {
                    if app.intro_state.is_none() {
                        handle_paste(app, &text);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_paste(app: &mut App, text: &str) {
    if let Some(ref mut picker) = app.path_picker {
        if picker.is_entering_custom {
            picker.custom_input.push_str(text);
            picker.custom_error = None;
        }
    } else if app.is_searching {
        app.search_query.push_str(text);
    }
}

fn handle_path_picker_key(app: &mut App, code: KeyCode) {
    if let Some(ref mut picker) = app.path_picker {
        if picker.is_entering_custom {
            match code {
                KeyCode::Enter => {
                    let input = picker.custom_input.clone();
                    if let Some(resolved) = crate::core::paths::resolve_user_path(&input) {
                        app.select_path(resolved);
                    } else if let Some(p) = app.path_picker.as_mut() {
                        p.custom_error =
                            Some("Directory does not exist or is inaccessible".to_string());
                    }
                }
                KeyCode::Esc => {
                    picker.is_entering_custom = false;
                    picker.custom_error = None;
                }
                KeyCode::Backspace => {
                    picker.custom_input.pop();
                    picker.custom_error = None;
                }
                KeyCode::Char(c) => {
                    picker.custom_input.push(c);
                    picker.custom_error = None;
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if picker.selected_index > 0 {
                    picker.selected_index -= 1;
                } else {
                    picker.selected_index = picker.items.len().saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if picker.selected_index + 1 < picker.items.len() {
                    picker.selected_index += 1;
                } else {
                    picker.selected_index = 0;
                }
            }
            KeyCode::PageUp => {
                picker.selected_index = picker.selected_index.saturating_sub(5);
            }
            KeyCode::PageDown => {
                if !picker.items.is_empty() {
                    picker.selected_index = (picker.selected_index + 5).min(picker.items.len() - 1);
                }
            }
            KeyCode::Home => {
                picker.selected_index = 0;
            }
            KeyCode::End => {
                picker.selected_index = picker.items.len().saturating_sub(1);
            }
            KeyCode::Char(c @ '1'..='9') => {
                if let Some(idx) = picker.items.iter().position(|it| it.shortcut == Some(c)) {
                    picker.selected_index = idx;
                    handle_picker_select(app);
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if let Some(idx) = picker
                    .items
                    .iter()
                    .position(|it| matches!(it.target, crate::ui::app::PathPickerTarget::CustomInput))
                {
                    picker.selected_index = idx;
                }
                picker.is_entering_custom = true;
                picker.custom_error = None;
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                if let Some(idx) = picker
                    .items
                    .iter()
                    .position(|it| matches!(it.target, crate::ui::app::PathPickerTarget::GlobalCaches))
                {
                    picker.selected_index = idx;
                }
                app.path_picker = None;
                app.active_tab = crate::ui::app::ActiveTab::GlobalCaches;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                handle_picker_select(app);
            }
            KeyCode::Esc => {
                if app.has_scanned {
                    app.path_picker = None;
                } else {
                    app.should_quit = true;
                }
            }
            KeyCode::Char('q') => {
                app.should_quit = true;
            }
            _ => {}
        }
    }
}

fn handle_picker_select(app: &mut App) {
    if let Some(ref picker) = app.path_picker {
        if let Some(item) = picker.items.get(picker.selected_index) {
            match &item.target {
                crate::ui::app::PathPickerTarget::Path(p) => {
                    let path = p.clone();
                    app.select_path(path);
                }
                crate::ui::app::PathPickerTarget::CustomInput => {
                    if let Some(p) = app.path_picker.as_mut() {
                        p.is_entering_custom = true;
                        p.custom_error = None;
                    }
                }
                crate::ui::app::PathPickerTarget::GlobalCaches => {
                    app.path_picker = None;
                    app.active_tab = crate::ui::app::ActiveTab::GlobalCaches;
                }
            }
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Check for Ctrl+C
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.cancel_deletion();
        app.should_quit = true;
        return;
    }

    if app.path_picker.is_some() {
        handle_path_picker_key(app, code);
        return;
    }

    // Modal state handling
    match app.deletion_state {
        DeletionState::Confirming => match code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.perform_deletion(DeleteMode::Trash);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                app.perform_deletion(DeleteMode::Permanent);
            }
            KeyCode::Esc | KeyCode::Char('c') => {
                app.deletion_state = DeletionState::Idle;
            }
            _ => {}
        },
        DeletionState::Done { .. } => match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.deletion_state = DeletionState::Idle;
            }
            _ => {}
        },
        DeletionState::Deleting { .. } => match code {
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                app.cancel_deletion();
            }
            _ => {}
        },
        DeletionState::Idle => {
            if app.is_searching {
                match code {
                    KeyCode::Enter => {
                        app.is_searching = false;
                    }
                    KeyCode::Esc => {
                        app.search_query.clear();
                        app.is_searching = false;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                    }
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc => {
                        if !app.search_query.is_empty() {
                            app.search_query.clear();
                        }
                    }
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.switch_tab();
                    }
                    KeyCode::Char('1') => {
                        app.active_tab = crate::ui::app::ActiveTab::Projects;
                    }
                    KeyCode::Char('2') => {
                        app.active_tab = crate::ui::app::ActiveTab::GlobalCaches;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.move_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.move_down();
                    }
                    KeyCode::PageUp => {
                        app.page_up(10);
                    }
                    KeyCode::PageDown => {
                        app.page_down(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        app.move_to_top();
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        app.move_to_bottom();
                    }
                    KeyCode::Char(' ') => {
                        app.toggle_selection();
                    }
                    KeyCode::Enter | KeyCode::Char('e') => {
                        app.toggle_expand();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.expand_group();
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.collapse_group();
                    }
                    KeyCode::Char('E') => {
                        app.toggle_expand_all();
                    }
                    KeyCode::Char('a') => {
                        app.toggle_all();
                    }
                    KeyCode::Char('s') => {
                        app.cycle_sort();
                    }
                    KeyCode::Char('/') => {
                        app.is_searching = true;
                    }
                    KeyCode::Char('p') => {
                        app.open_path_picker();
                    }
                    KeyCode::Char('r') => {
                        app.start_scan();
                        app.start_global_cache_scan();
                    }
                    KeyCode::Char('d') => {
                        let (selected_count, _) = app.get_selected_stats();
                        if selected_count == 0 {
                            // Automatically select current item if nothing is selected
                            app.toggle_selection();
                        }
                        let (new_count, _) = app.get_selected_stats();
                        if new_count > 0 {
                            app.deletion_state = DeletionState::Confirming;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
