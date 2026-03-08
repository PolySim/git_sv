//! Chargement de la configuration utilisateur.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

/// Configuration utilisateur de `git_sv`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Langue de l'interface.
    #[serde(default)]
    pub language: Language,
}

impl AppConfig {
    /// Charge la configuration depuis le fichier utilisateur.
    pub fn load() -> Result<Self> {
        Self::load_from_path(&Self::config_path())
    }

    /// Retourne le chemin du fichier de configuration.
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("git_sv")
            .join("config.json")
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_config_returns_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("missing.json");

        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config.language, Language::Fr);
    }

    #[test]
    fn test_load_config_with_english_language() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        std::fs::write(&path, r#"{"language":"en"}"#).unwrap();

        let config = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(config.language, Language::En);
    }
}
