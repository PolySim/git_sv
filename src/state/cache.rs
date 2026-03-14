//! Cache LRU pour les diffs de fichiers et lazy loading.

use git2::Oid;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::git::diff::FileDiff;

/// Clé de cache pour un diff.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffCacheKey {
    /// OID du commit (ou OID zéro pour working directory)
    pub commit_oid: Oid,
    /// Chemin du fichier
    pub file_path: String,
}

impl DiffCacheKey {
    #[cfg(test)]
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
    cache: LruCache<DiffCacheKey, FileDiff>,
}

impl DiffCache {
    /// Crée un nouveau cache avec la capacité donnée.
    pub fn new(capacity: usize) -> Self {
        const DEFAULT_CAPACITY: NonZeroUsize = NonZeroUsize::new(1).unwrap();
        let cap = NonZeroUsize::new(capacity).unwrap_or(DEFAULT_CAPACITY);
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Récupère un diff du cache (et le marque comme récemment utilisé).
    pub fn get(&mut self, key: &DiffCacheKey) -> Option<&FileDiff> {
        self.cache.get(key)
    }

    /// Insère un diff dans le cache.
    pub fn put(&mut self, key: DiffCacheKey, diff: FileDiff) {
        self.cache.put(key, diff);
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
        // LruCache ne permet pas de supprimer par prédicat facilement,
        // donc on reconstruit le cache sans les entrées WD
        let _capacity = self.cache.cap();
        let entries: Vec<_> = self
            .cache
            .iter()
            .filter(|(k, _)| !k.is_working_dir())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        self.cache.clear();
        for (k, v) in entries {
            self.cache.put(k, v);
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
        self.cache.cap().get()
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
}
