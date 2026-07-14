//! Mapping clavier et souris vers les actions de l'application.
//!
//! Ce module est le point central de gestion des entrees utilisateur.

mod keyboard;
mod mouse;
#[cfg(test)]
mod tests;

use crossterm::event::{self, Event};
use std::time::Duration;

use crate::state::{AppAction, AppState};

use keyboard::map_key;

#[cfg(test)]
pub(crate) use keyboard::map_key_for_test;
use mouse::map_mouse;

/// Résultat d'un cycle d'attente d'entrée terminal.
pub struct InputPoll {
    pub action: Option<AppAction>,
    pub event_received: bool,
}

/// Poll un evenement avec un timeout configurable (clavier + souris).
pub fn handle_input_with_timeout(
    state: &AppState,
    timeout: Duration,
) -> std::io::Result<InputPoll> {
    if event::poll(timeout)? {
        let action = match event::read()? {
            Event::Key(key) => map_key(key, state),
            Event::Mouse(mouse) => map_mouse(mouse, state),
            _ => None,
        };
        Ok(InputPoll {
            action,
            event_received: true,
        })
    } else {
        Ok(InputPoll {
            action: None,
            event_received: false,
        })
    }
}
