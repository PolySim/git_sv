# STEP 06 - Amélioration du DiffCache et Performances

**Priorité**: 🟡 Moyenne  
**Effort estimé**: 2-3 heures  
**Risque**: Faible  
**Prérequis**: STEP_01 à STEP_05 complétés

---

## Objectif

Améliorer les performances de l'application en :
1. Remplaçant l'implémentation custom de `DiffCache` par la crate `lru`
2. Optimisant les opérations O(n) identifiées
3. Ajoutant du lazy loading pour les données lourdes
4. Réduisant les allocations inutiles

---

## 1. Problèmes de performance identifiés

### 1.1 DiffCache actuel (O(n) sur chaque accès)

```rust
// Implémentation actuelle - PROBLÉMATIQUE
pub struct DiffCache {
    cache: HashMap<(git2::Oid, String), FileDiff>,
    access_order: Vec<(git2::Oid, String)>,  // ← O(n) pour find + remove
    max_size: usize,
}

impl DiffCache {
    pub fn get(&mut self, key: &(git2::Oid, String)) -> Option<&FileDiff> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            let key = self.access_order.remove(pos);  // ← O(n)
            self.access_order.push(key);              // ← O(1)
        }
        self.cache.get(key)
    }
}
```

**Complexité actuelle**: O(n) pour chaque `get()`

### 1.2 Vec::remove(0) dans certains handlers

```rust
// Patterns problématiques identifiés
self.some_vec.remove(0);  // O(n) - devrait utiliser VecDeque
```

### 1.3 Clonages excessifs

```rust
// Pattern répété
let path = entry.path.clone();  // Clone parfois inutile
```

### 1.4 Rechargement complet du graph

À chaque modification, tout le graph est rechargé au lieu d'une mise à jour incrémentale.

---

## 2. Nouveau DiffCache avec crate `lru`

### Mise à jour de `Cargo.toml`

```toml
[dependencies]
# ... existants ...
lru = "0.12"
```

### Fichier: `src/state/cache.rs`

```rust
//! Cache LRU pour les diffs de fichiers.

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
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
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
    pub fn contains(&self, key: &DiffCacheKey) -> bool {
        self.cache.contains(key)
    }

    /// Vide le cache complètement.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Invalide toutes les entrées du working directory.
    /// 
    /// Appelé après stage/unstage/commit pour s'assurer que
    /// les diffs du working directory sont rechargés.
    pub fn clear_working_directory(&mut self) {
        // LruCache ne permet pas de supprimer par prédicat facilement,
        // donc on reconstruit le cache sans les entrées WD
        let capacity = self.cache.cap();
        let entries: Vec<_> = self.cache
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
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Le cache est-il vide?
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Capacité du cache.
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
        
        let diff = FileDiff::default();
        
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
        
        let diff = FileDiff::default();
        
        cache.put(wd_key.clone(), diff.clone());
        cache.put(commit_key.clone(), diff);
        
        assert_eq!(cache.len(), 2);
        
        cache.clear_working_directory();
        
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(&wd_key));
        assert!(cache.contains(&commit_key));
    }
}
```

---

## 3. Optimisation des collections

### 3.1 Utiliser `VecDeque` pour les insertions/suppressions en tête

```rust
// AVANT
let mut items: Vec<T> = ...;
items.remove(0);  // O(n)

// APRÈS
use std::collections::VecDeque;
let mut items: VecDeque<T> = ...;
items.pop_front();  // O(1)
```

### 3.2 Utiliser `Cow` pour éviter les clones

```rust
use std::borrow::Cow;

// AVANT
fn process_path(&self, path: String) {
    // path est cloné même si pas nécessaire
}

// APRÈS
fn process_path(&self, path: Cow<'_, str>) {
    // Clone seulement si mutation nécessaire
}
```

### 3.3 Pré-allouer les vecteurs

```rust
// AVANT
let mut items = Vec::new();
for i in 0..expected_size {
    items.push(compute(i));
}

// APRÈS
let mut items = Vec::with_capacity(expected_size);
for i in 0..expected_size {
    items.push(compute(i));
}
```

---

## 4. Lazy loading pour les données lourdes

### 4.1 Diff chargé à la demande

```rust
/// État d'un diff (chargé paresseusement).
pub enum LazyDiff {
    NotLoaded,
    Loading,
    Loaded(FileDiff),
    Error(String),
}

impl LazyDiff {
    pub fn get_or_load<F>(&mut self, loader: F) -> Option<&FileDiff>
    where
        F: FnOnce() -> Result<FileDiff, String>,
    {
        if matches!(self, LazyDiff::NotLoaded) {
            *self = LazyDiff::Loading;
            match loader() {
                Ok(diff) => *self = LazyDiff::Loaded(diff),
                Err(e) => *self = LazyDiff::Error(e),
            }
        }
        
        match self {
            LazyDiff::Loaded(diff) => Some(diff),
            _ => None,
        }
    }
}
```

### 4.2 Blame chargé à la demande

```rust
// Dans BlameState
pub struct BlameState {
    pub file_path: String,
    pub blame: LazyBlame,
    pub selected_line: usize,
    pub scroll_offset: usize,
}

pub enum LazyBlame {
    NotLoaded,
    Loading,
    Loaded(FileBlame),
    Error(String),
}
```

---

## 5. Optimisation du graph

### 5.1 Mise à jour incrémentale

Au lieu de recharger tout le graph après chaque commit:

```rust
// Dans helpers.rs
pub fn refresh_graph_incremental(state: &mut AppState, new_commits: usize) -> Result<()> {
    // Charger uniquement les nouveaux commits
    let new_entries = state.repo.build_graph_partial(new_commits)?;
    
    // Fusionner avec l'existant
    let mut graph = std::mem::take(&mut state.graph);
    for entry in new_entries.into_iter().rev() {
        graph.items.insert(0, entry);
    }
    
    // Limiter la taille
    while graph.len() > MAX_COMMITS {
        graph.items.pop();
    }
    
    state.graph = graph;
    Ok(())
}
```

### 5.2 Virtualisation de la liste

Pour les très longs historiques, ne charger que les commits visibles + marge:

```rust
/// Graph virtualisé pour les gros repos.
pub struct VirtualizedGraph {
    /// Commits actuellement chargés.
    loaded: Vec<GraphRow>,
    /// Index du premier commit chargé dans l'historique complet.
    start_index: usize,
    /// Nombre total de commits dans le repo.
    total_count: usize,
    /// Marge de commits à garder chargés autour de la sélection.
    buffer_size: usize,
}

impl VirtualizedGraph {
    /// S'assure que les commits autour de l'index sont chargés.
    pub fn ensure_loaded(&mut self, index: usize, repo: &GitRepo) -> Result<()> {
        let buffer = self.buffer_size;
        let desired_start = index.saturating_sub(buffer);
        let desired_end = (index + buffer).min(self.total_count);
        
        if desired_start < self.start_index || desired_end > self.start_index + self.loaded.len() {
            // Recharger la fenêtre
            self.loaded = repo.build_graph_range(desired_start, desired_end - desired_start)?;
            self.start_index = desired_start;
        }
        
        Ok(())
    }
}
```

---

## 6. Profilage et mesure

### 6.1 Ajouter des timers de debug

```rust
// Dans Cargo.toml
[features]
profiling = []

// Dans le code
#[cfg(feature = "profiling")]
macro_rules! time_block {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        eprintln!("[PERF] {} took {:?}", $name, start.elapsed());
        result
    }};
}

#[cfg(not(feature = "profiling"))]
macro_rules! time_block {
    ($name:expr, $block:expr) => {
        $block
    };
}

// Usage
fn refresh_graph(&mut self) -> Result<()> {
    time_block!("build_graph", {
        self.state.graph = self.state.repo.build_graph(MAX_COMMITS)?;
    });
    Ok(())
}
```

### 6.2 Script de benchmark

```bash
#!/bin/bash
# scripts/benchmark.sh

echo "Benchmarking git_sv..."

# Repo de test (linux kernel pour stress test)
REPO="/tmp/linux"

if [ ! -d "$REPO" ]; then
    echo "Cloning linux kernel (this will take a while)..."
    git clone --depth 1000 https://github.com/torvalds/linux.git "$REPO"
fi

# Benchmark startup time
echo "Startup time:"
time timeout 5 cargo run --release -- --path "$REPO" log -n 1

# Benchmark avec profiling
echo "With profiling:"
cargo run --release --features profiling -- --path "$REPO" log -n 100 2>&1 | grep PERF
```

---

## 7. Résumé des optimisations

| Optimisation | Complexité avant | Complexité après | Impact |
|--------------|-----------------|------------------|--------|
| DiffCache.get() | O(n) | O(1) | Élevé |
| Vec::remove(0) | O(n) | O(1) avec VecDeque | Moyen |
| Clone évités | - | - | Faible |
| Lazy loading | Tout chargé | À la demande | Élevé |
| Graph virtualisé | O(total) | O(visible) | Élevé |

---

## 8. Checklist de validation

```bash
# 1. Ajouter la dépendance lru
cargo add lru

# 2. Compiler
cargo build

# 3. Tests du cache
cargo test cache

# 4. Tests complets
cargo test

# 5. Benchmark informel
cargo build --release
time ./target/release/git_sv --path /path/to/big/repo log -n 1

# 6. Profiling (optionnel)
cargo run --release --features profiling 2>&1 | grep PERF
```

---

## Notes

- L'optimisation du graph virtualisé est optionnelle et ne devrait être implémentée que si nécessaire pour les très gros repos
- Le lazy loading améliore le temps de démarrage mais peut causer des micro-freezes lors du premier accès
- Toujours mesurer avant/après pour valider les gains
