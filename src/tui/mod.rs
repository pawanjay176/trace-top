mod app;
mod keymap;
mod screens;
mod terminal;

use std::{
    error::Error,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use std::sync::Arc;

use crate::core::store::Store;

use self::{
    app::{Action, AppState, Screen},
    terminal::TerminalSession,
};

const TRACE_LIST_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

pub fn run(store: Arc<Store>) -> Result<(), Box<dyn Error>> {
    let mut terminal = TerminalSession::enter()?;
    let result = run_app(&mut terminal, store);
    terminal.exit()?;
    result
}

fn run_app(terminal: &mut TerminalSession, store: Arc<Store>) -> Result<(), Box<dyn Error>> {
    let mut app = AppState::new();
    app.update(Action::RefreshTraceList, &store);
    let mut last_trace_list_refresh = Instant::now();

    loop {
        if app.screen == Screen::TraceList
            && last_trace_list_refresh.elapsed() >= TRACE_LIST_REFRESH_INTERVAL
        {
            app.update(Action::RefreshTraceList, &store);
            last_trace_list_refresh = Instant::now();
        }

        terminal.draw(|frame| screens::render(frame, &mut app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = keymap::map_key(key, &app);
                    app.update(action, &store);
                }
                CrosstermEvent::Resize(width, height) => {
                    app.update(Action::Resize(width, height), &store);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
