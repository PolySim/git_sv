//! Builder pour les blocs stylisés.

use ratatui::widgets::{Block, Borders};
use super::style::{border_style, title_style};

/// Builder pour créer des blocs avec un style cohérent.
pub struct StyledBlock {
    title: String,
    is_focused: bool,
    borders: Borders,
}

impl StyledBlock {
    /// Crée un nouveau builder de bloc.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            is_focused: false,
            borders: Borders::ALL,
        }
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Définit les bordures à afficher.
    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Construit le widget Block.
    pub fn build(self) -> Block<'static> {
        Block::default()
            .title(self.title)
            .title_style(title_style())
            .borders(self.borders)
            .border_style(border_style(self.is_focused))
    }
}
