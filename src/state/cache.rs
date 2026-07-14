//! Cache LRU pour les diffs de fichiers et lazy loading.

use git2::Oid;
use lru::LruCache;
use std::sync::Arc;

use crate::git::diff::FileDiff;

/// Taille mémoire maximale conservée par le cache de diffs.
const DEFAULT_MAX_BYTES: usize = 64 * 1_048_576;

/// Etat d'une ressource chargee paresseusement.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub enum Lazy<T> {
    /// Non charge encore.
    #[default]
    NotLoaded,
    /// En cours de chargement.
    Loading,
    /// Charge avec succes.
    Loaded(T),
    /// Erreur de chargement.
    Error(String),
}

#[allow(dead_code)]
impl<T> Lazy<T> {
    /// Cree un nouvel etat `NotLoaded`.
    pub fn new() -> Self {
        Self::NotLoaded
    }

    /// Recupere la valeur si chargee, ou la charge via le loader fourni.
    pub fn get_or_load<F>(&mut self, loader: F) -> Option<&T>
    where
        F: FnOnce() -> crate::error::Result<T>,
    {
        if matches!(self, Self::NotLoaded) {
            *self = Self::Loading;
            match loader() {
                Ok(value) => *self = Self::Loaded(value),
                Err(e) => *self = Self::Error(e.to_string()),
            }
        }

        match self {
            Self::Loaded(value) => Some(value),
            _ => None,
        }
    }

    /// Force le rechargement.
    pub fn reload<F>(&mut self, loader: F) -> Option<&T>
    where
        F: FnOnce() -> crate::error::Result<T>,
    {
        *self = Self::NotLoaded;
        self.get_or_load(loader)
    }

    /// Verifie si la ressource est chargee.
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// Verifie si la ressource est en cours de chargement.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Recupere la valeur si chargee, sans tenter de charger.
    pub fn get(&self) -> Option<&T> {
        match self {
            Self::Loaded(value) => Some(value),
            _ => None,
        }
    }

    /// Reinitialise l'etat a `NotLoaded`.
    pub fn reset(&mut self) {
        *self = Self::NotLoaded;
    }
}

#[allow(dead_code)]
/// Alias pour un diff charge paresseusement.
pub type LazyDiff = Lazy<FileDiff>;

#[allow(dead_code)]
/// Alias pour un blame charge paresseusement.
pub type LazyBlame = Lazy<crate::git::blame::FileBlame>;

/// Clé de cache pour un diff.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffCacheKey {
    /// OID du commit (ou OID zéro pour working directory)
    pub commit_oid: Oid,
    /// Chemin du fichier
    pub file_path: String,
}

impl DiffCacheKey {
    pub fn new(commit_oid: Oid, file_path: impl Into<String>) -> Self {
        Self {
            commit_oid,
            file_path: file_path.into(),
        }
    }

    /// Clé pour un fichier du working directory.
    pub fn working_dir(file_path: impl Into<String>) -> Self {
        Self {
            commit_oid: Oid::zero(),
            file_path: file_path.into(),
        }
    }

    /// Est-ce une clé working directory?
    pub fn is_working_dir(&self) -> bool {
        self.commit_oid.is_zero()
    }
}

/// Cache LRU pour les diffs de fichiers.
pub struct DiffCache {
    cache: LruCache<DiffCacheKey, Arc<FileDiff>>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
}

impl DiffCache {
    /// Crée un nouveau cache avec la capacité donnée.
    pub fn new(capacity: usize) -> Self {
        Self::with_limits(capacity, DEFAULT_MAX_BYTES)
    }

    fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            cache: LruCache::unbounded(),
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1),
            current_bytes: 0,
        }
    }

    /// Récupère un diff du cache (et le marque comme récemment utilisé).
    pub fn get(&mut self, key: &DiffCacheKey) -> Option<Arc<FileDiff>> {
        self.cache.get(key).cloned()
    }

    /// Insère un diff dans le cache.
    pub fn put(&mut self, key: DiffCacheKey, diff: impl Into<Arc<FileDiff>>) {
        if let Some(previous) = self.cache.pop(&key) {
            self.current_bytes = self
                .current_bytes
                .saturating_sub(previous.estimated_memory_bytes());
        }

        let diff = diff.into();
        self.current_bytes = self
            .current_bytes
            .saturating_add(diff.estimated_memory_bytes());
        self.cache.put(key, diff);
        self.enforce_limits();
    }

    fn enforce_limits(&mut self) {
        while self.cache.len() > self.max_entries || self.current_bytes > self.max_bytes {
            let Some((_, diff)) = self.cache.pop_lru() else {
                self.current_bytes = 0;
                break;
            };
            self.current_bytes = self
                .current_bytes
                .saturating_sub(diff.estimated_memory_bytes());
        }
    }

    /// Vérifie si une clé est présente.
    #[cfg(test)]
    pub fn contains(&self, key: &DiffCacheKey) -> bool {
        self.cache.contains(key)
    }

    /// Invalide toutes les entrées du working directory.
    ///
    /// Appelé après stage/unstage/commit pour s'assurer que
    /// les diffs du working directory sont rechargés.
    pub fn clear_working_directory(&mut self) {
        let keys = self
            .cache
            .iter()
            .filter(|(key, _)| key.is_working_dir())
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();

        for key in keys {
            if let Some(diff) = self.cache.pop(&key) {
                self.current_bytes = self
                    .current_bytes
                    .saturating_sub(diff.estimated_memory_bytes());
            }
        }
    }

    /// Nombre d'entrées dans le cache.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Capacité du cache.
    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.max_entries
    }

    #[cfg(test)]
    pub fn memory_bytes(&self) -> usize {
        self.current_bytes
    }
}

impl Default for DiffCache {
    fn default() -> Self {
        Self::new(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oid(n: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        Oid::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn test_lazy_get_or_load_success() {
        let mut lazy = Lazy::new();

        let value = lazy.get_or_load(|| Ok::<_, crate::error::GitSvError>(42));

        assert_eq!(value, Some(&42));
        assert!(lazy.is_loaded());
        assert_eq!(lazy.get(), Some(&42));
    }

    #[test]
    fn test_lazy_get_or_load_error() {
        let mut lazy = Lazy::<u32>::new();

        let value = lazy.get_or_load(|| Err(crate::error::GitSvError::Other("echec".into())));

        assert!(value.is_none());
        assert!(matches!(lazy, Lazy::Error(ref message) if message == "echec"));
    }

    #[test]
    fn test_lazy_reload_replaces_existing_value() {
        let mut lazy = Lazy::Loaded(1);

        let value = lazy.reload(|| Ok::<_, crate::error::GitSvError>(2));

        assert_eq!(value, Some(&2));
        assert_eq!(lazy.get(), Some(&2));
    }

    #[test]
    fn test_lazy_reset_clears_loaded_state() {
        let mut lazy = Lazy::Loaded(7);

        lazy.reset();

        assert!(matches!(lazy, Lazy::NotLoaded));
        assert!(!lazy.is_loaded());
        assert!(!lazy.is_loading());
        assert!(lazy.get().is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = DiffCache::new(2);

        let key1 = DiffCacheKey::new(make_oid(1), "file1.rs");
        let key2 = DiffCacheKey::new(make_oid(2), "file2.rs");
        let key3 = DiffCacheKey::new(make_oid(3), "file3.rs");

        let diff = FileDiff {
            path: String::new(),
            status: crate::git::diff::DiffStatus::Modified,
            lines: Vec::new(),
            additions: 0,
            deletions: 0,
            image_preview: None,
        };

        cache.put(key1.clone(), diff.clone());
        cache.put(key2.clone(), diff.clone());

        assert!(cache.contains(&key1));
        assert!(cache.contains(&key2));

        // Accéder à key1 pour le rendre récent
        cache.get(&key1);

        // Ajouter key3 devrait évincer key2 (le moins récent)
        cache.put(key3.clone(), diff);

        assert!(cache.contains(&key1));
        assert!(!cache.contains(&key2));
        assert!(cache.contains(&key3));
    }

    #[test]
    fn test_clear_working_directory() {
        let mut cache = DiffCache::new(10);

        let wd_key = DiffCacheKey::working_dir("file.rs");
        let commit_key = DiffCacheKey::new(make_oid(1), "file.rs");

        let diff = FileDiff {
            path: String::new(),
            status: crate::git::diff::DiffStatus::Modified,
            lines: Vec::new(),
            additions: 0,
            deletions: 0,
            image_preview: None,
        };

        cache.put(wd_key.clone(), diff.clone());
        cache.put(commit_key.clone(), diff);

        assert_eq!(cache.len(), 2);

        cache.clear_working_directory();

        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(&wd_key));
        assert!(cache.contains(&commit_key));
    }

    #[test]
    fn test_cache_key_working_dir() {
        let wd_key = DiffCacheKey::working_dir("test.rs");
        assert!(wd_key.is_working_dir());
        assert_eq!(wd_key.commit_oid, Oid::zero());
        assert_eq!(wd_key.file_path, "test.rs");

        let commit_key = DiffCacheKey::new(make_oid(1), "test.rs");
        assert!(!commit_key.is_working_dir());
    }

    #[test]
    fn test_default_capacity() {
        let cache = DiffCache::default();
        assert_eq!(cache.capacity(), 50);
    }

    #[test]
    fn test_cache_shares_diff_without_deep_clone() {
        let mut cache = DiffCache::new(2);
        let key = DiffCacheKey::new(make_oid(1), "large.rs");
        let diff = Arc::new(FileDiff {
            path: "large.rs".to_string(),
            status: crate::git::diff::DiffStatus::Modified,
            lines: vec![crate::git::diff::DiffLine {
                line_type: crate::git::diff::DiffLineType::Context,
                content: "contenu".repeat(100),
                old_lineno: Some(1),
                new_lineno: Some(1),
            }],
            additions: 0,
            deletions: 0,
            image_preview: None,
        });

        cache.put(key.clone(), diff.clone());
        let cached = cache.get(&key).expect("diff en cache");

        assert!(Arc::ptr_eq(&diff, &cached));
    }

    #[test]
    fn test_cache_evicts_entries_above_memory_budget() {
        let mut cache = DiffCache::with_limits(10, 256);
        let diff = FileDiff {
            path: "large.rs".to_string(),
            status: crate::git::diff::DiffStatus::Modified,
            lines: vec![crate::git::diff::DiffLine {
                line_type: crate::git::diff::DiffLineType::Context,
                content: "x".repeat(1_024),
                old_lineno: Some(1),
                new_lineno: Some(1),
            }],
            additions: 0,
            deletions: 0,
            image_preview: None,
        };

        cache.put(DiffCacheKey::new(make_oid(1), "large.rs"), diff);

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.memory_bytes(), 0);
    }
}
