//! Composant de base pour les popups et dialogues.

use super::{rect::centered_rect, style::border_style};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Configuration pour un popup.
pub struct Popup<'a> {
    title: &'a str,
    content: Vec<Line<'a>>,
    width_percent: u16,
    height_percent: u16,
    is_focused: bool,
}

impl<'a> Popup<'a> {
    /// Crée un nouveau popup.
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            content: Vec::new(),
            width_percent: 60,
            height_percent: 40,
            is_focused: true,
        }
    }

    /// Définit le contenu du popup.
    pub fn content(mut self, content: Vec<Line<'a>>) -> Self {
        self.content = content;
        self
    }

    /// Définit la taille en pourcentage.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width_percent = width;
        self.height_percent = height;
        self
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Rend le popup dans le frame.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(self.width_percent, self.height_percent, area);

        // Clear le fond
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(border_style(self.is_focused));

        let paragraph = Paragraph::new(self.content)
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, popup_area);
    }

    /// Retourne la zone du popup pour un rendu personnalisé.
    pub fn area(&self, parent: Rect) -> Rect {
        centered_rect(self.width_percent, self.height_percent, parent)
    }
}
