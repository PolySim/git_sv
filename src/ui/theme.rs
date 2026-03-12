//! Configuration des thèmes et couleurs.

use ratatui::style::Color;
use std::sync::OnceLock;

use crate::config::ThemeMode;

/// Thème de couleurs pour l'application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Couleur primaire (bordures, éléments actifs)
    pub primary: Color,
    /// Couleur secondaire (éléments secondaires)
    pub secondary: Color,
    /// Couleur de fond des éléments sélectionnés
    pub selection_bg: Color,
    /// Couleur du texte sélectionné
    pub selection_fg: Color,
    /// Couleur des bordures inactives
    pub border_inactive: Color,
    /// Couleur des bordures actives
    pub border_active: Color,
    /// Couleur de la status bar
    pub status_bar_bg: Color,
    /// Couleur du texte de la status bar
    pub status_bar_fg: Color,
    /// Couleur des messages d'erreur
    pub error: Color,
    /// Couleur des messages de succès
    pub success: Color,
    /// Couleur des avertissements
    pub warning: Color,
    /// Couleur des informations
    pub info: Color,
    /// Couleur du hash des commits
    pub commit_hash: Color,
    /// Couleur du texte normal
    pub text_normal: Color,
    /// Couleur du texte secondaire (dates, métadonnées)
    pub text_secondary: Color,
    /// Couleur de fond générale
    pub background: Color,
    /// Couleur de fond pour "ours" (conflits)
    pub ours_bg: Color,
    /// Couleur de fond pour "theirs" (conflits)
    pub theirs_bg: Color,
}

impl Theme {
    /// Thème sombre (défaut).
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Magenta,
            selection_bg: Color::DarkGray,
            selection_fg: Color::White,
            border_inactive: Color::Gray,
            border_active: Color::Cyan,
            status_bar_bg: Color::Cyan,
            status_bar_fg: Color::Black,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            info: Color::Blue,
            commit_hash: Color::Yellow,
            text_normal: Color::White,
            text_secondary: Color::Gray,
            background: Color::Black,
            ours_bg: Color::Indexed(22),   // Vert très foncé
            theirs_bg: Color::Indexed(17), // Bleu très foncé
        }
    }

    /// Thème clair.
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Magenta,
            selection_bg: Color::Indexed(252), // Gris clair visible sur fond blanc
            selection_fg: Color::Black,
            border_inactive: Color::DarkGray,
            border_active: Color::Blue,
            status_bar_bg: Color::Blue,
            status_bar_fg: Color::White,
            error: Color::Red,
            success: Color::Indexed(28), // Vert foncé lisible sur fond clair
            warning: Color::Indexed(130), // Orange/brun lisible sur fond clair
            info: Color::Indexed(26),    // Bleu foncé
            commit_hash: Color::Indexed(130), // Orange/brun lisible sur fond clair
            text_normal: Color::Black,
            text_secondary: Color::DarkGray,
            background: Color::White,
            ours_bg: Color::Indexed(194),   // Vert très clair
            theirs_bg: Color::Indexed(189), // Bleu très clair
        }
    }

    /// Construit le thème à partir du mode configuré.
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    /// Couleurs assignées aux branches du graphe (thème sombre).
    const BRANCH_COLORS_DARK: &[Color] = &[
        Color::Green,
        Color::Red,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::LightGreen,
        Color::LightRed,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
    ];

    /// Couleurs assignées aux branches du graphe (thème clair).
    /// Utilise des couleurs plus saturées/foncées pour être lisibles sur fond blanc.
    const BRANCH_COLORS_LIGHT: &[Color] = &[
        Color::Indexed(28),  // Vert foncé
        Color::Indexed(124), // Rouge foncé
        Color::Indexed(130), // Orange/brun
        Color::Indexed(26),  // Bleu foncé
        Color::Indexed(127), // Magenta foncé
        Color::Indexed(30),  // Cyan foncé
        Color::Indexed(34),  // Vert moyen
        Color::Indexed(160), // Rouge moyen
        Color::Indexed(166), // Orange
        Color::Indexed(32),  // Bleu moyen
        Color::Indexed(133), // Magenta moyen
        Color::Indexed(36),  // Cyan moyen
    ];

    /// Retourne la couleur pour un index de branche selon le thème actuel.
    pub fn branch_color(&self, index: usize) -> Color {
        let colors = if self.background == Color::White {
            Self::BRANCH_COLORS_LIGHT
        } else {
            Self::BRANCH_COLORS_DARK
        };
        colors[index % colors.len()]
    }
}

/// Stockage global du thème (initialisé une seule fois au démarrage).
static THEME: OnceLock<Theme> = OnceLock::new();

/// Initialise le thème global à partir de la configuration.
/// Doit être appelé une seule fois au démarrage, avant tout accès au thème.
pub fn init_theme(mode: ThemeMode) {
    let theme = Theme::from_mode(mode);
    THEME.get_or_init(|| theme);
}

/// Retourne le thème actuel.
/// Renvoie le thème sombre par défaut si `init_theme` n'a pas été appelé.
pub fn current_theme() -> &'static Theme {
    THEME.get_or_init(Theme::dark)
}

/// Retourne la couleur pour un index de branche.
pub fn branch_color(index: usize) -> Color {
    current_theme().branch_color(index)
}
