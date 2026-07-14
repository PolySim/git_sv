//! Chargement de la configuration utilisateur.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

/// Mode de thème de l'application.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Thème sombre.
    #[default]
    Dark,
    /// Thème clair.
    Light,
    /// Thème utilisant la palette Solarized du terminal.
    Solarized,
}

impl ThemeMode {
    /// Liste ordonnée des thèmes proposés par la CLI.
    pub const ALL: [Self; 3] = [Self::Dark, Self::Light, Self::Solarized];

    /// Nom utilisé dans le fichier de configuration et la CLI.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Solarized => "solarized",
        }
    }
}

/// Configuration utilisateur de `git_sv`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Langue de l'interface.
    #[serde(default)]
    pub language: Language,
    /// Thème de couleurs (dark, light ou solarized).
    #[serde(default)]
    pub theme: ThemeMode,
    /// Remplacement de raccourcis, indexé par identifiant d'action.
    #[serde(default)]
    pub keybindings: BTreeMap<String, String>,
    /// Commandes shell déclenchées par un raccourci global.
    #[serde(default)]
    pub custom_commands: Vec<CustomCommandConfig>,
}

/// Commande utilisateur exécutée hors de la TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCommandConfig {
    pub name: String,
    pub key: String,
    pub command: String,
    /// Demander une confirmation avant l'exécution.
    #[serde(default = "default_true")]
    pub confirm: bool,
    /// Attendre Entrée avant de revenir à la TUI.
    #[serde(default = "default_true")]
    pub pause: bool,
}

const fn default_true() -> bool {
    true
}

/// Raccourcis précompilés utilisés pendant la boucle d'événements.
#[derive(Debug, Clone, Default)]
pub struct RuntimeCustomization {
    pub keybindings: Vec<ResolvedKeyBinding>,
    pub custom_commands: Vec<ResolvedCustomCommand>,
}

#[derive(Debug, Clone)]
pub struct ResolvedKeyBinding {
    pub action: String,
    pub chord: KeyChord,
}

#[derive(Debug, Clone)]
pub struct ResolvedCustomCommand {
    pub definition: CustomCommandConfig,
    pub chord: KeyChord,
}

/// Représentation compacte d'une combinaison clavier configurée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    /// Parse `ctrl+shift+x`, `alt+enter`, `pageup`, `space`, etc.
    pub fn parse(value: &str) -> Option<Self> {
        let parts = value
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let (key, modifier_parts) = parts.split_last()?;
        let mut modifiers = KeyModifiers::NONE;
        for modifier in modifier_parts {
            match modifier.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "alt" | "option" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "super" | "cmd" | "command" => modifiers |= KeyModifiers::SUPER,
                _ => return None,
            }
        }
        let normalized = key.to_ascii_lowercase();
        let code = match normalized.as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "space" => KeyCode::Char(' '),
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdown" => KeyCode::PageDown,
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            _ if key.chars().count() == 1 => KeyCode::Char(key.chars().next()?),
            _ => return None,
        };
        Some(Self { code, modifiers })
    }

    pub fn matches(&self, event: KeyEvent) -> bool {
        const RELEVANT: KeyModifiers = KeyModifiers::CONTROL
            .union(KeyModifiers::ALT)
            .union(KeyModifiers::SHIFT)
            .union(KeyModifiers::SUPER);
        let code_matches = match (self.code, event.code) {
            (KeyCode::Char(expected), KeyCode::Char(actual))
                if self.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                expected.eq_ignore_ascii_case(&actual)
            }
            _ => self.code == event.code,
        };
        code_matches && self.modifiers == event.modifiers.intersection(RELEVANT)
    }
}

impl AppConfig {
    /// Précompile les combinaisons valides une seule fois au démarrage.
    pub fn runtime_customization(&self) -> RuntimeCustomization {
        RuntimeCustomization {
            keybindings: self
                .keybindings
                .iter()
                .filter_map(|(action, key)| {
                    KeyChord::parse(key).map(|chord| ResolvedKeyBinding {
                        action: action.clone(),
                        chord,
                    })
                })
                .collect(),
            custom_commands: self
                .custom_commands
                .iter()
                .filter(|definition| {
                    !definition.name.trim().is_empty() && !definition.command.trim().is_empty()
                })
                .filter_map(|definition| {
                    KeyChord::parse(&definition.key).map(|chord| ResolvedCustomCommand {
                        definition: definition.clone(),
                        chord,
                    })
                })
                .collect(),
        }
    }

    /// Charge la configuration depuis le fichier utilisateur.
    pub fn load() -> Result<Self> {
        for path in Self::candidate_paths() {
            if path.exists() {
                return Self::load_from_path(&path);
            }
        }

        let config = Self::default();
        config.write_default_file()?;
        Ok(config)
    }

    /// Retourne les chemins de configuration pris en charge.
    pub fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".config").join("git_sv").join("config.json"));
        }

        if let Some(config_dir) = dirs::config_dir() {
            let platform_path = config_dir.join("git_sv").join("config.json");
            if !paths.iter().any(|path| path == &platform_path) {
                paths.push(platform_path);
            }
        }

        paths
    }

    /// Charge la configuration depuis un chemin donné.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Ecrit le fichier de configuration par defaut au premier chemin supporte.
    pub fn write_default_file(&self) -> Result<()> {
        let Some(path) = Self::candidate_paths().into_iter().next() else {
            return Ok(());
        };

        self.write_default_file_to_path(&path)
    }

    /// Ecrit le fichier de configuration par defaut a un chemin donne.
    pub fn write_default_file_to_path(&self, path: &Path) -> Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }

        if !path.exists() {
            self.save_to_path(path)?;
        }

        Ok(())
    }

    /// Sauvegarde la configuration dans le fichier actuellement utilisé.
    pub fn save(&self) -> Result<PathBuf> {
        let candidates = Self::candidate_paths();
        let path = candidates
            .iter()
            .find(|path| path.exists())
            .cloned()
            .or_else(|| candidates.into_iter().next())
            .ok_or_else(|| anyhow!("Aucun chemin de configuration disponible"))?;

        self.save_to_path(&path)?;
        Ok(path)
    }

    /// Ecrit la configuration complète dans un chemin donné.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, format!("{}\n", content))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_config_returns_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("missing.json");

        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config.language, Language::En);
    }

    #[test]
    fn test_load_config_with_english_language() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        std::fs::write(&path, r#"{"language":"en"}"#).unwrap();

        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config.language, Language::En);
    }

    #[test]
    fn test_load_config_with_solarized_theme() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        std::fs::write(&path, r#"{"theme":"solarized"}"#).unwrap();

        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config.theme, ThemeMode::Solarized);
    }

    #[test]
    fn test_theme_names_are_stable() {
        let names: Vec<_> = ThemeMode::ALL.iter().map(|theme| theme.as_str()).collect();
        assert_eq!(names, ["dark", "light", "solarized"]);
    }

    #[test]
    fn test_candidate_paths_contains_xdg_style_path() {
        let paths = AppConfig::candidate_paths();
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with(".config/git_sv/config.json")),
            "Le chemin ~/.config/git_sv/config.json devrait etre pris en charge"
        );
    }

    #[test]
    fn test_write_default_file_creates_english_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let config = AppConfig::default();

        config.write_default_file_to_path(&path).unwrap();

        let loaded = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.language, Language::En);
    }

    #[test]
    fn test_save_to_path_updates_existing_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"language":"en","theme":"dark"}"#).unwrap();

        let config = AppConfig {
            language: Language::Fr,
            theme: ThemeMode::Solarized,
            ..AppConfig::default()
        };
        config.save_to_path(&path).unwrap();

        let loaded = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.language, Language::Fr);
        assert_eq!(loaded.theme, ThemeMode::Solarized);
    }

    #[test]
    fn test_key_chord_parses_modifiers_and_named_keys() {
        let chord = KeyChord::parse("ctrl+shift+x").unwrap();
        assert!(chord.matches(KeyEvent::new(
            KeyCode::Char('X'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )));
        assert_eq!(KeyChord::parse("pageup").unwrap().code, KeyCode::PageUp);
        assert!(KeyChord::parse("ctrl+unknown-key").is_none());
    }

    #[test]
    fn test_runtime_customization_ignores_invalid_chords() {
        let config = AppConfig {
            keybindings: BTreeMap::from([
                ("global.help".to_string(), "ctrl+h".to_string()),
                ("global.quit".to_string(), "not-a-key".to_string()),
            ]),
            custom_commands: vec![CustomCommandConfig {
                name: "Tests".to_string(),
                key: "alt+t".to_string(),
                command: "cargo test".to_string(),
                confirm: true,
                pause: false,
            }],
            ..AppConfig::default()
        };

        let runtime = config.runtime_customization();
        assert_eq!(runtime.keybindings.len(), 1);
        assert_eq!(runtime.custom_commands.len(), 1);
    }
}
