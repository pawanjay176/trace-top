mod app;
mod keymap;
mod screens;
mod terminal;

use std::{error::Error, time::Duration};

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::core::store::{Store, StoreEvent};

use self::{
    app::{Action, AppState},
    terminal::TerminalSession,
};

pub fn run(
    store: Arc<Store>,
    store_rx: mpsc::Receiver<Vec<StoreEvent>>,
) -> Result<(), Box<dyn Error>> {
    let mut terminal = TerminalSession::enter()?;
    let result = run_app(&mut terminal, store, store_rx);
    terminal.exit()?;
    result
}

fn run_app(
    terminal: &mut TerminalSession,
    store: Arc<Store>,
    mut store_rx: mpsc::Receiver<Vec<StoreEvent>>,
) -> Result<(), Box<dyn Error>> {
    let mut app = AppState::new();

    loop {
        while let Ok(events) = store_rx.try_recv() {
            for event in events {
                app.update(Action::StoreChanged(event), &store);
            }
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
