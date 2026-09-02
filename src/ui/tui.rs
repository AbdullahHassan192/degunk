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

        terminal.draw(|f| {
            let total_area = f.area();

            let constraints = if app.is_searching {
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

            if app.is_searching {
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

        // Poll keyboard events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key.code, key.modifiers);
                }
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Check for Ctrl+C
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.should_quit = true;
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
        DeletionState::Deleting { .. } => {}
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
                    KeyCode::Char('r') => {
                        app.start_scan();
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
