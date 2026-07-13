//! Configuration des thèmes et couleurs.

use ratatui::style::Color;
use std::sync::OnceLock;

use crate::config::ThemeMode;

/// Thème de couleurs pour l'application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Mode du thème configuré.
    pub mode: ThemeMode,
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
    /// Couleur des surfaces structurelles (panneaux, barres).
    pub surface: Color,
    /// Couleur des surfaces secondaires ou surélevées.
    pub surface_alt: Color,
    /// Couleur de fond pour "ours" (conflits)
    pub ours_bg: Color,
    /// Couleur de fond pour "theirs" (conflits)
    pub theirs_bg: Color,
}

impl Theme {
    /// Thème sombre (défaut).
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            primary: Color::Rgb(99, 197, 218),
            secondary: Color::Rgb(210, 168, 255),
            selection_bg: Color::Rgb(28, 58, 82),
            selection_fg: Color::Rgb(244, 247, 250),
            border_inactive: Color::Rgb(86, 97, 109),
            border_active: Color::Rgb(99, 197, 218),
            status_bar_bg: Color::Rgb(23, 33, 43),
            status_bar_fg: Color::Rgb(230, 237, 243),
            error: Color::Rgb(255, 107, 107),
            success: Color::Rgb(79, 209, 123),
            warning: Color::Rgb(232, 180, 76),
            info: Color::Rgb(108, 182, 255),
            commit_hash: Color::Rgb(230, 197, 106),
            text_normal: Color::Rgb(230, 237, 243),
            text_secondary: Color::Rgb(169, 180, 192),
            background: Color::Rgb(11, 15, 20),
            surface: Color::Rgb(17, 24, 32),
            surface_alt: Color::Rgb(23, 33, 43),
            ours_bg: Color::Rgb(18, 55, 37),
            theirs_bg: Color::Rgb(59, 31, 36),
        }
    }

    /// Thème clair.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            primary: Color::Rgb(6, 122, 145),
            secondary: Color::Rgb(122, 79, 179),
            selection_bg: Color::Rgb(207, 232, 243),
            selection_fg: Color::Rgb(11, 37, 51),
            border_inactive: Color::Rgb(122, 133, 146),
            border_active: Color::Rgb(6, 122, 145),
            status_bar_bg: Color::Rgb(232, 237, 242),
            status_bar_fg: Color::Rgb(23, 33, 43),
            error: Color::Rgb(180, 35, 45),
            success: Color::Rgb(24, 122, 67),
            warning: Color::Rgb(122, 79, 0),
            info: Color::Rgb(29, 99, 181),
            commit_hash: Color::Rgb(111, 66, 193),
            text_normal: Color::Rgb(23, 33, 43),
            text_secondary: Color::Rgb(76, 89, 103),
            background: Color::Rgb(245, 247, 250),
            surface: Color::Rgb(255, 255, 255),
            surface_alt: Color::Rgb(232, 237, 242),
            ours_bg: Color::Rgb(221, 245, 230),
            theirs_bg: Color::Rgb(252, 227, 229),
        }
    }

    /// Thème Solarized piloté par la palette ANSI du terminal.
    ///
    /// Les fonds et le texte principal utilisent `Reset` pour conserver les
    /// couleurs du profil terminal, aussi bien en Solarized Light que Dark.
    pub fn solarized() -> Self {
        Self {
            mode: ThemeMode::Solarized,
            primary: Color::Cyan,
            secondary: Color::LightMagenta,
            selection_bg: Color::LightCyan,
            selection_fg: Color::Black,
            border_inactive: Color::LightGreen,
            border_active: Color::Cyan,
            status_bar_bg: Color::Reset,
            status_bar_fg: Color::Reset,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            info: Color::Blue,
            commit_hash: Color::LightMagenta,
            text_normal: Color::Reset,
            text_secondary: Color::LightGreen,
            background: Color::Reset,
            surface: Color::Reset,
            surface_alt: Color::Reset,
            ours_bg: Color::Reset,
            theirs_bg: Color::Reset,
        }
    }

    /// Construit le thème à partir du mode configuré.
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::Solarized => Self::solarized(),
        }
    }

    /// Retourne true si le thème est en mode clair.
    #[allow(dead_code)] // Prevu pour d'autres contextes de rendu.
    pub fn is_light(&self) -> bool {
        self.mode == ThemeMode::Light
    }

    /// Couleurs assignées aux branches du graphe (thème sombre).
    const BRANCH_COLORS_DARK: &[Color] = &[
        Color::Rgb(79, 209, 123),
        Color::Rgb(255, 107, 107),
        Color::Rgb(232, 180, 76),
        Color::Rgb(108, 182, 255),
        Color::Rgb(210, 168, 255),
        Color::Rgb(99, 197, 218),
        Color::Rgb(86, 211, 100),
        Color::Rgb(255, 123, 114),
        Color::Rgb(230, 197, 106),
        Color::Rgb(121, 192, 255),
        Color::Rgb(188, 140, 255),
        Color::Rgb(86, 212, 221),
    ];

    /// Couleurs assignées aux branches du graphe (thème clair).
    /// Utilise des couleurs plus saturées/foncées pour être lisibles sur fond blanc.
    const BRANCH_COLORS_LIGHT: &[Color] = &[
        Color::Rgb(24, 122, 67),
        Color::Rgb(180, 35, 45),
        Color::Rgb(122, 79, 0),
        Color::Rgb(29, 99, 181),
        Color::Rgb(111, 66, 193),
        Color::Rgb(6, 122, 145),
        Color::Rgb(17, 99, 41),
        Color::Rgb(164, 14, 38),
        Color::Rgb(125, 78, 0),
        Color::Rgb(5, 80, 174),
        Color::Rgb(102, 57, 186),
        Color::Rgb(5, 105, 107),
    ];

    /// Couleurs de branches déléguées aux emplacements ANSI Solarized.
    const BRANCH_COLORS_SOLARIZED: &[Color] = &[
        Color::Green,
        Color::Red,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
        Color::LightMagenta,
        Color::LightBlue,
        Color::LightCyan,
        Color::LightGreen,
        Color::DarkGray,
    ];

    /// Retourne la couleur pour un index de branche selon le thème actuel.
    pub fn branch_color(&self, index: usize) -> Color {
        let colors = match self.mode {
            ThemeMode::Light => Self::BRANCH_COLORS_LIGHT,
            ThemeMode::Dark => Self::BRANCH_COLORS_DARK,
            ThemeMode::Solarized => Self::BRANCH_COLORS_SOLARIZED,
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
    if THEME.set(theme).is_err() {
        eprintln!("Warning: init_theme called more than once, ignoring.");
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn relative_luminance(color: Color) -> f64 {
        let Color::Rgb(red, green, blue) = color else {
            panic!("Les palettes doivent utiliser des couleurs RGB déterministes");
        };
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
    }

    fn contrast_ratio(first: Color, second: Color) -> f64 {
        let first = relative_luminance(first);
        let second = relative_luminance(second);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    fn assert_theme_contrast(theme: Theme) {
        for color in [
            theme.text_normal,
            theme.text_secondary,
            theme.primary,
            theme.secondary,
            theme.error,
            theme.success,
            theme.warning,
            theme.info,
            theme.commit_hash,
        ] {
            assert!(
                contrast_ratio(color, theme.background) >= 4.5,
                "contraste insuffisant pour {color:?} sur {:?}",
                theme.background
            );
        }
        assert!(contrast_ratio(theme.selection_fg, theme.selection_bg) >= 4.5);
        assert!(contrast_ratio(theme.status_bar_fg, theme.status_bar_bg) >= 4.5);
        assert!(contrast_ratio(theme.border_inactive, theme.background) >= 3.0);

        for index in 0..12 {
            assert!(contrast_ratio(theme.branch_color(index), theme.background) >= 4.5);
        }
    }

    #[test]
    fn test_dark_theme_contrast() {
        assert_theme_contrast(Theme::dark());
    }

    #[test]
    fn test_light_theme_contrast() {
        assert_theme_contrast(Theme::light());
    }

    #[test]
    fn test_solarized_theme_uses_terminal_palette() {
        let theme = Theme::solarized();

        assert_eq!(theme.background, Color::Reset);
        assert_eq!(theme.surface, Color::Reset);
        assert_eq!(theme.surface_alt, Color::Reset);
        assert_eq!(theme.text_normal, Color::Reset);
        assert_eq!(theme.selection_bg, Color::LightCyan);
        assert_eq!(theme.selection_fg, Color::Black);

        for index in 0..12 {
            assert!(!matches!(theme.branch_color(index), Color::Rgb(..)));
        }
    }
}
