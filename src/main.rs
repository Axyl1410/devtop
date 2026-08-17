mod app;
mod system;
mod ui;

use app::{ActiveTab, App, SortBy};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::stdout,
    panic,
    time::Duration,
};

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

    // Modal popup keys
    if app.show_kill_confirm {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_kill(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_kill(),
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
                app.set_status("Filter cleared");
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Tab | KeyCode::Right => app.next_tab(),
        KeyCode::BackTab | KeyCode::Left => app.prev_tab(),
        KeyCode::Char('1') => app.active_tab = ActiveTab::Overview,
        KeyCode::Char('2') => app.active_tab = ActiveTab::Processes,
        KeyCode::Char('3') => app.active_tab = ActiveTab::StorageNetwork,
        KeyCode::Char('4') | KeyCode::Char('?') => app.active_tab = ActiveTab::Help,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.active_tab == ActiveTab::Processes {
                app.prev_process();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.active_tab == ActiveTab::Processes {
                let len = app.filtered_sorted_processes().len();
                app.next_process(len);
            }
        }
        KeyCode::Char('/') => {
            app.active_tab = ActiveTab::Processes;
            app.search_mode = true;
        }
        KeyCode::Char('s') => {
            let next_sort = match app.sort_by {
                SortBy::Cpu => SortBy::Memory,
                SortBy::Memory => SortBy::Pid,
                SortBy::Pid => SortBy::Name,
                SortBy::Name => SortBy::Cpu,
            };
            app.toggle_sort(next_sort);
        }
        KeyCode::Char('c') => app.toggle_sort(SortBy::Cpu),
        KeyCode::Char('m') => app.toggle_sort(SortBy::Memory),
        KeyCode::Char('p') => app.toggle_sort(SortBy::Pid),
        KeyCode::Char('n') => app.toggle_sort(SortBy::Name),
        KeyCode::Char('x') => {
            if app.active_tab == ActiveTab::Processes {
                app.request_kill_selected();
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_refresh_rate(),
        KeyCode::Char('-') | KeyCode::Char('_') => app.decrease_refresh_rate(),
        _ => {}
    }
}
