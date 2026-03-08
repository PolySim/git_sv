//! Internationalisation simple pour `git_sv`.

use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// Langues supportées par l'application.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Français.
    #[default]
    Fr,
    /// Anglais.
    En,
}

fn language_store() -> &'static RwLock<Language> {
    static STORE: OnceLock<RwLock<Language>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(Language::default()))
}

/// Définit la langue courante de l'application.
pub fn set_language(language: Language) {
    if let Ok(mut current) = language_store().write() {
        *current = language;
    }
}

/// Retourne la langue courante de l'application.
pub fn current_language() -> Language {
    language_store()
        .read()
        .map(|guard| *guard)
        .unwrap_or_default()
}

/// Retourne une chaîne localisée statique.
pub fn text<'a>(fr: &'a str, en: &'a str) -> &'a str {
    match current_language() {
        Language::Fr => fr,
        Language::En => en,
    }
}

/// Retourne une chaîne localisée allouée.
pub fn text_owned(fr: impl Into<String>, en: impl Into<String>) -> String {
    match current_language() {
        Language::Fr => fr.into(),
        Language::En => en.into(),
    }
}

#[cfg(test)]
pub fn with_language<T>(language: Language, f: impl FnOnce() -> T) -> T {
    use std::sync::Mutex;

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    let lock = TEST_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap();

    let previous = current_language();
    set_language(language);
    let result = f();
    set_language(previous);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_in_french() {
        with_language(Language::Fr, || {
            assert_eq!(text("Bonjour", "Hello"), "Bonjour");
        });
    }

    #[test]
    fn test_text_in_english() {
        with_language(Language::En, || {
            assert_eq!(text("Bonjour", "Hello"), "Hello");
        });
    }
}
