//! Recherche et défilement locaux à l'aide, sans accès au dépôt.

const MAX_QUERY_CHARS: usize = 256;

/// Action propre au popup d'aide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpAction {
    StartSearch,
    StopSearch,
    Insert(String),
    Backspace,
    ClearQuery,
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// État éphémère réinitialisé à chaque ouverture de l'aide.
#[derive(Debug, Default)]
pub struct HelpState {
    pub(crate) searching: bool,
    pub(crate) query: String,
    pub(crate) offset: usize,
    page_height: usize,
    max_offset: usize,
}

impl HelpState {
    /// Borne le défilement avec les dimensions effectivement rendues.
    pub(crate) fn set_viewport(&mut self, rows: usize, height: usize) {
        self.page_height = height;
        self.max_offset = if height == 0 {
            0
        } else {
            rows.saturating_sub(height)
        };
        self.offset = self.offset.min(self.max_offset);
    }

    pub(crate) fn apply(&mut self, action: HelpAction) {
        match action {
            HelpAction::StartSearch => self.searching = true,
            HelpAction::StopSearch => self.searching = false,
            HelpAction::Up => self.offset = self.offset.saturating_sub(1),
            HelpAction::Down => self.offset = (self.offset + 1).min(self.max_offset),
            HelpAction::PageUp => self.offset = self.offset.saturating_sub(self.page_height),
            HelpAction::PageDown => {
                self.offset = (self.offset + self.page_height).min(self.max_offset)
            }
            HelpAction::Top => self.offset = 0,
            HelpAction::Bottom => self.offset = self.max_offset,
            _ if !self.searching => {}
            HelpAction::Insert(value) => {
                let remaining = MAX_QUERY_CHARS - self.query.chars().count();
                self.query.extend(
                    value
                        .chars()
                        .filter_map(|c| {
                            if c.is_whitespace() {
                                Some(' ')
                            } else if c.is_control() {
                                None
                            } else {
                                Some(c)
                            }
                        })
                        .take(remaining),
                );
                self.offset = 0;
            }
            HelpAction::Backspace => {
                self.query.pop();
                self.offset = 0;
            }
            HelpAction::ClearQuery => {
                self.query.clear();
                self.offset = 0;
            }
        }
    }
}
