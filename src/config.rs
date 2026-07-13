//! Chargement de la configuration utilisateur.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
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
}

impl AppConfig {
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
        };
        config.save_to_path(&path).unwrap();

        let loaded = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.language, Language::Fr);
        assert_eq!(loaded.theme, ThemeMode::Solarized);
    }
}
