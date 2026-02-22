//! Cache LRU pour les diffs de fichiers.

use crate::git::diff::FileDiff;
use std::collections::HashMap;

/// Cache LRU simple pour les diffs de fichiers.
/// Clé: (Oid du commit, chemin du fichier)
/// Valeur: FileDiff
pub struct DiffCache {
    cache: HashMap<(git2::Oid, String), FileDiff>,
    /// Ordre d'accès pour LRU (dernier = plus récent).
    access_order: Vec<(git2::Oid, String)>,
    /// Taille maximale du cache.
    max_size: usize,
}

impl DiffCache {
    /// Crée un nouveau cache avec une taille maximale.
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_size,
        }
    }

    /// Récupère un diff du cache.
    pub fn get(&mut self, key: &(git2::Oid, String)) -> Option<&FileDiff> {
        if self.cache.contains_key(key) {
            // Mettre à jour l'ordre d'accès (LRU)
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key = self.access_order.remove(pos);
                self.access_order.push(key);
            }
            self.cache.get(key)
        } else {
            None
        }
    }

    /// Insère un diff dans le cache.
    pub fn insert(&mut self, key: (git2::Oid, String), value: FileDiff) {
        // Si la clé existe déjà, mettre à jour juste la valeur
        if self.cache.contains_key(&key) {
            self.cache.insert(key.clone(), value);
            // Mettre à jour l'ordre d'accès
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                let key = self.access_order.remove(pos);
                self.access_order.push(key);
            }
            return;
        }

        // Éviction LRU si nécessaire
        if self.cache.len() >= self.max_size && !self.access_order.is_empty() {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.cache.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        self.cache.insert(key.clone(), value);
        self.access_order.push(key);
    }

    /// Vide le cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Supprime les entrées du working directory (Oid::zero()).
    pub fn clear_working_directory(&mut self) {
        let to_remove: Vec<_> = self
            .cache
            .keys()
            .filter(|(oid, _)| *oid == git2::Oid::zero())
            .cloned()
            .collect();
        for key in to_remove {
            self.cache.remove(&key);
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                self.access_order.remove(pos);
            }
        }
    }
}
