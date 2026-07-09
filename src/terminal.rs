//! Initialisation et restauration du terminal crossterm.
//!
//! Ce module gère le cycle de vie du terminal TUI :
//! activation du mode raw, passage en alternate screen,
//! capture de la souris, et restauration à la sortie.

use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

use crate::error::Result;

/// Session terminal restaurée automatiquement à la sortie.
pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhancement_enabled: bool,
    restored: bool,
}

impl TerminalSession {
    /// Initialise le terminal et retourne une session RAII.
    pub fn setup() -> Result<Self> {
        let (terminal, keyboard_enhancement_enabled) = setup_terminal()?;
        Ok(Self {
            terminal,
            keyboard_enhancement_enabled,
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
            restore_terminal(&mut self.terminal, self.keyboard_enhancement_enabled)?;
            self.restored = true;
        }
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.restored {
            let _ = restore_terminal(&mut self.terminal, self.keyboard_enhancement_enabled);
            self.restored = true;
        }
    }
}

/// Initialise le terminal en mode raw + alternate screen + mouse capture.
fn setup_terminal() -> Result<(Terminal<CrosstermBackend<Stdout>>, bool)> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let keyboard_enhancement_enabled = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    if keyboard_enhancement_enabled {
        execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )?;
    }
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok((terminal, keyboard_enhancement_enabled))
}

/// Restaure le terminal à son état normal.
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhancement_enabled: bool,
) -> Result<()> {
    if keyboard_enhancement_enabled {
        execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}
