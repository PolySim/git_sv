//! Composant liste stylisé et réutilisable.

use super::{block::StyledBlock, style::highlight_style};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem, ListState},
    Frame,
};

/// Configuration pour une liste stylisée.
pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    is_focused: bool,
    selected: Option<usize>,
}

impl<'a> StyledList<'a> {
    /// Crée une nouvelle liste.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            items: Vec::new(),
            title: title.into(),
            is_focused: false,
            selected: None,
        }
    }

    /// Définit les éléments de la liste.
    pub fn items(mut self, items: Vec<ListItem<'a>>) -> Self {
        self.items = items;
        self
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Définit l'index sélectionné.
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Rend la liste dans le frame.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let block = StyledBlock::new(&self.title)
            .focused(self.is_focused)
            .build();

        let list = List::new(self.items)
            .block(block)
            .highlight_style(highlight_style());

        let mut state = ListState::default().with_selected(self.selected);
        frame.render_stateful_widget(list, area, &mut state);
    }
}

/// Helper pour créer des ListItem avec style cohérent.
pub fn list_item(content: impl Into<String>) -> ListItem<'static> {
    ListItem::new(content.into())
}

pub fn list_item_styled(content: impl Into<String>, style: Style) -> ListItem<'static> {
    ListItem::new(content.into()).style(style)
}
