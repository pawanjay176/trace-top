use std::{
    error::Error,
    io::{self, Stdout},
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    exited: bool,
}

impl TerminalSession {
    pub fn enter() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            exited: false,
        })
    }

    pub fn draw<F>(&mut self, render: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(render)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        if self.exited {
            return Ok(());
        }
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        self.exited = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}
