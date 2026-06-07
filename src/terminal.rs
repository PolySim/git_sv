//! Initialisation et restauration du terminal crossterm.
//!
//! Ce module gère le cycle de vie du terminal TUI :
//! activation du mode raw, passage en alternate screen,
//! capture de la souris, et restauration à la sortie.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

use crate::error::Result;

/// Session terminal restaurée automatiquement à la sortie.
pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    restored: bool,
}

impl TerminalSession {
    /// Initialise le terminal et retourne une session RAII.
    pub fn setup() -> Result<Self> {
        Ok(Self {
            terminal: setup_terminal()?,
            restored: false,
        })
    }

    /// Retourne un accès mutable au terminal.
    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Restaure explicitement le terminal.
    pub fn restore(&mut self) -> Result<()> {
        if !self.restored {
            restore_terminal(&mut self.terminal)?;
            self.restored = true;
        }
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.restored {
            let _ = restore_terminal(&mut self.terminal);
            self.restored = true;
        }
    }
}

/// Initialise le terminal en mode raw + alternate screen + mouse capture.
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restaure le terminal à son état normal.
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
