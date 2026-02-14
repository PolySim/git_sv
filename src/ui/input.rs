use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::app::{App, AppAction};

/// Poll un événement clavier et retourne l'action correspondante.
pub fn handle_input(app: &App) -> std::io::Result<Option<AppAction>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            return Ok(map_key(key, app));
        }
    }
    Ok(None)
}

/// Mappe un événement clavier à une action de l'application.
fn map_key(key: KeyEvent, _app: &App) -> Option<AppAction> {
    // Ctrl+C quitte toujours.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(AppAction::Quit);
    }

    match key.code {
        // Navigation
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
        KeyCode::Enter => Some(AppAction::Select),

        // Actions git
        KeyCode::Char('c') => Some(AppAction::CommitPrompt),
        KeyCode::Char('s') => Some(AppAction::StashPrompt),
        KeyCode::Char('m') => Some(AppAction::MergePrompt),
        KeyCode::Char('b') => Some(AppAction::BranchList),

        // Aide
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Rafraîchir
        KeyCode::Char('r') => Some(AppAction::Refresh),

        // Basculer entre les modes du panneau bas-gauche
        KeyCode::Tab => Some(AppAction::SwitchBottomMode),

        _ => None,
    }
}
