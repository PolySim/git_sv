//! Configuration des thèmes et couleurs.

use ratatui::style::Color;

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
            text_secondary: Color::DarkGray,
            background: Color::Black,
        }
    }

    /// Thème clair.
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Magenta,
            selection_bg: Color::Gray,
            selection_fg: Color::Black,
            border_inactive: Color::DarkGray,
            border_active: Color::Blue,
            status_bar_bg: Color::Blue,
            status_bar_fg: Color::White,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            info: Color::Cyan,
            commit_hash: Color::Yellow,
            text_normal: Color::Black,
            text_secondary: Color::DarkGray,
            background: Color::White,
        }
    }

    /// Couleurs assignées aux branches du graphe.
    pub const BRANCH_COLORS: &[Color] = &[
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

    /// Retourne la couleur pour un index de branche.
    pub fn branch_color(index: usize) -> Color {
        Self::BRANCH_COLORS[index % Self::BRANCH_COLORS.len()]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Thème global de l'application (statique pour l'instant).
/// Pourrait être chargé depuis un fichier de configuration plus tard.
pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::default);

/// Retourne le thème actuel.
pub fn current_theme() -> &'static Theme {
    &THEME
}

/// Retourne la couleur pour un index de branche.
pub fn branch_color(index: usize) -> Color {
    Theme::branch_color(index)
}

/// Change le thème (nécessite un redémarrage pour l'instant).
pub fn set_theme(theme: Theme) {
    // Note: Pour un vrai changement à chaud, il faudrait utiliser une Mutex
    // ou un autre mécanisme de synchronisation.
    // Pour l'instant, le thème est fixé au démarrage.
    let _ = theme;
}
