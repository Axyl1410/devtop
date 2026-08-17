mod app;
mod core;
mod ui;

use app::{ActiveTab, App, SortBy};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io::stdout, panic, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup panic hook to restore terminal on unexpected panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let res = run_app(&mut terminal, &mut app);

    // Teardown terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(app, f))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key.code, key.modifiers);
                }
            }
        }

        app.on_tick();
    }

    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Handle Ctrl+C globally
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Modal popup keys (Signal Selection & Kill Confirmation)
    if app.show_kill_confirm {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Left | KeyCode::Char('h') => {
                app.prev_signal();
            }
            KeyCode::Down
            | KeyCode::Char('j')
            | KeyCode::Right
            | KeyCode::Char('l')
            | KeyCode::Tab => {
                app.next_signal();
            }
            KeyCode::BackTab => {
                app.prev_signal();
            }
            KeyCode::Char('1') => app.select_signal_by_index(0), // 15: SIGTERM
            KeyCode::Char('2') => app.select_signal_by_index(1), // 9: SIGKILL
            KeyCode::Char('3') => app.select_signal_by_index(2), // 2: SIGINT
            KeyCode::Char('4') => app.select_signal_by_index(3), // 1: SIGHUP
            KeyCode::Char('5') => app.select_signal_by_index(4), // 19: SIGSTOP
            KeyCode::Char('6') => app.select_signal_by_index(5), // 18: SIGCONT
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_kill(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                app.cancel_kill()
            }
            _ => {}
        }
        return;
    }

    // Process Detail drill-down mode keys
    if app.show_process_detail {
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace => {
                app.close_process_detail();
            }
            KeyCode::Char('k') | KeyCode::Char('x') => {
                app.request_kill_selected();
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.should_quit = true;
            }
            _ => {}
        }
        return;
    }

    // Search Mode input
    if app.search_mode {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.search_mode = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        return;
    }

    // Normal Mode
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.set_status("Search filter cleared");
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Tab | KeyCode::Right => app.next_tab(),
        KeyCode::BackTab | KeyCode::Left => app.prev_tab(),
        KeyCode::Char('1') => app.select_tab(0),
        KeyCode::Char('2') => app.select_tab(1),
        KeyCode::Char('3') => app.select_tab(2),
        KeyCode::Char('4') => app.select_tab(3),
        KeyCode::Char('5') | KeyCode::Char('?') => app.select_tab(4),
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_item();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_item();
        }
        KeyCode::PageUp => {
            app.page_up(10);
        }
        KeyCode::PageDown => {
            app.page_down(10);
        }
        KeyCode::Home | KeyCode::Char('g') => match app.active_tab {
            ActiveTab::Processes => {
                app.selected_process = 0;
                app.scroll_offset = 0;
            }
            ActiveTab::Ports => {
                app.selected_port = 0;
                app.port_scroll_offset = 0;
            }
            _ => {}
        },
        KeyCode::End | KeyCode::Char('G') => match app.active_tab {
            ActiveTab::Processes => {
                let len = if app.tree_mode {
                    app.tree_processes().len()
                } else {
                    app.filtered_sorted_processes().len()
                };
                if len > 0 {
                    app.selected_process = len - 1;
                }
            }
            ActiveTab::Ports => {
                let len = app.filtered_ports().len();
                if len > 0 {
                    app.selected_port = len - 1;
                }
            }
            _ => {}
        },
        KeyCode::Enter => {
            app.open_process_detail();
        }
        KeyCode::Char('t') => {
            if app.active_tab == ActiveTab::Processes {
                app.toggle_tree_mode();
            }
        }
        KeyCode::Char('/') => {
            if app.active_tab == ActiveTab::Overview || app.active_tab == ActiveTab::Help {
                app.active_tab = ActiveTab::Processes;
            }
            app.search_mode = true;
        }
        KeyCode::Char('s') => {
            let next_sort = match app.sort_by {
                SortBy::Cpu => SortBy::Memory,
                SortBy::Memory => SortBy::Pid,
                SortBy::Pid => SortBy::Name,
                SortBy::Name => SortBy::User,
                SortBy::User => SortBy::Cpu,
            };
            app.toggle_sort(next_sort);
        }
        KeyCode::Char('c') => app.toggle_sort(SortBy::Cpu),
        KeyCode::Char('m') => app.toggle_sort(SortBy::Memory),
        KeyCode::Char('p') => app.toggle_sort(SortBy::Pid),
        KeyCode::Char('n') => app.toggle_sort(SortBy::Name),
        KeyCode::Char('u') => app.toggle_sort(SortBy::User),
        KeyCode::Char('x') | KeyCode::Char('K') => {
            app.request_kill_selected();
        }
        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_refresh_rate(),
        KeyCode::Char('-') | KeyCode::Char('_') => app.decrease_refresh_rate(),
        _ => {}
    }
}
