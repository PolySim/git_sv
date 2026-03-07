//! Mapping clavier et souris vers les actions de l'application.
//!
//! Ce module est le point central de gestion des entrees utilisateur.

#![allow(dead_code)]

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

/// Timeout par defaut pour le polling des evenements (ms).
const DEFAULT_INPUT_TIMEOUT_MS: u64 = 100;

/// Poll un evenement clavier et retourne l'action correspondante.
pub fn handle_input(state: &AppState) -> std::io::Result<Option<AppAction>> {
    handle_input_with_timeout(state, DEFAULT_INPUT_TIMEOUT_MS)
}

/// Poll un evenement avec un timeout configurable (clavier + souris).
pub fn handle_input_with_timeout(
    state: &AppState,
    timeout_ms: u64,
) -> std::io::Result<Option<AppAction>> {
    if event::poll(Duration::from_millis(timeout_ms))? {
        match event::read()? {
            Event::Key(key) => Ok(map_key(key, state)),
            Event::Mouse(mouse) => Ok(map_mouse(mouse, state)),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}
