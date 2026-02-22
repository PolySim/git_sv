//! Barre d'aide configurable.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Un raccourci clavier à afficher.
pub struct KeyBinding {
    pub key: &'static str,
    pub description: &'static str,
}

impl KeyBinding {
    pub const fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

/// Configuration de la barre d'aide.
pub struct HelpBar<'a> {
    bindings: &'a [KeyBinding],
    separator: &'a str,
}

impl<'a> HelpBar<'a> {
    /// Crée une nouvelle barre d'aide.
    pub fn new(bindings: &'a [KeyBinding]) -> Self {
        Self {
            bindings,
            separator: "  ",
        }
    }

    /// Définit le séparateur entre les bindings.
    pub fn separator(mut self, sep: &'a str) -> Self {
        self.separator = sep;
        self
    }

    /// Rend la barre d'aide.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        for (i, binding) in self.bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(self.separator));
            }

            spans.push(Span::styled(
                binding.key,
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(": "));
            spans.push(Span::raw(binding.description));
        }

        let help_line = Line::from(spans);
        let help_paragraph = Paragraph::new(help_line);

        frame.render_widget(help_paragraph, area);
    }
}

// Bindings communs réutilisables
pub mod bindings {
    use super::KeyBinding;

    pub const QUIT: KeyBinding = KeyBinding::new("q", "Quitter");
    pub const HELP: KeyBinding = KeyBinding::new("?", "Aide");
    pub const NAV_UP_DOWN: KeyBinding = KeyBinding::new("↑↓", "Naviguer");
    pub const ENTER: KeyBinding = KeyBinding::new("Enter", "Sélectionner");
    pub const ESC: KeyBinding = KeyBinding::new("Esc", "Retour");
    pub const TAB: KeyBinding = KeyBinding::new("Tab", "Changer panel");
}
