use git2::{Oid, Repository};

use super::commit::CommitInfo;
use crate::error::Result;

/// Noeud du graphe de commits, enrichi avec des infos de placement.
#[derive(Debug, Clone)]
pub struct CommitNode {
    /// OID du commit.
    pub oid: Oid,
    /// Message court du commit.
    pub message: String,
    /// Auteur du commit.
    pub author: String,
    /// Date du commit (timestamp unix).
    pub timestamp: i64,
    /// OIDs des parents.
    pub parents: Vec<Oid>,
    /// Noms des refs pointant vers ce commit (branches, tags).
    pub refs: Vec<String>,
    /// Colonne assignée pour le rendu du graphe.
    pub column: usize,
}

/// Construit le graphe de commits avec placement en colonnes.
///
/// L'algorithme assigne une colonne à chaque commit de manière à ce que
/// les branches parallèles occupent des colonnes distinctes.
pub fn build_graph(repo: &Repository, commits: &[CommitInfo]) -> Result<Vec<CommitNode>> {
    let mut nodes = Vec::with_capacity(commits.len());
    // Colonnes actives : chaque slot contient l'OID attendu dans cette colonne.
    let mut active_columns: Vec<Option<Oid>> = Vec::new();

    // Collecter les refs pour chaque OID.
    let refs_map = collect_refs(repo)?;

    for ci in commits {
        let oid = ci.oid;

        // Trouver la colonne de ce commit.
        let column = find_or_assign_column(&mut active_columns, oid);

        // Libérer la colonne courante.
        if column < active_columns.len() {
            active_columns[column] = None;
        }

        // Assigner les parents dans les colonnes.
        for (i, &parent_oid) in ci.parents.iter().enumerate() {
            if i == 0 {
                // Le premier parent prend la même colonne.
                if column < active_columns.len() {
                    active_columns[column] = Some(parent_oid);
                } else {
                    // Étendre si nécessaire.
                    while active_columns.len() <= column {
                        active_columns.push(None);
                    }
                    active_columns[column] = Some(parent_oid);
                }
            } else {
                // Les parents supplémentaires (merge) prennent une nouvelle colonne.
                assign_new_column(&mut active_columns, parent_oid);
            }
        }

        let refs = refs_map.get(&oid).cloned().unwrap_or_default();

        nodes.push(CommitNode {
            oid,
            message: ci.message.clone(),
            author: ci.author.clone(),
            timestamp: ci.timestamp,
            parents: ci.parents.clone(),
            refs,
            column,
        });
    }

    Ok(nodes)
}

/// Collecte toutes les références (branches, tags) et les associe à leur OID.
fn collect_refs(repo: &Repository) -> Result<std::collections::HashMap<Oid, Vec<String>>> {
    let mut map = std::collections::HashMap::new();

    for reference in repo.references()? {
        let reference = reference?;
        if let Some(name) = reference.shorthand() {
            if let Some(oid) = reference.target() {
                map.entry(oid)
                    .or_insert_with(Vec::new)
                    .push(name.to_string());
            }
        }
    }

    Ok(map)
}

/// Trouve la colonne existante pour un OID, ou en assigne une nouvelle.
fn find_or_assign_column(active_columns: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    // Chercher si cet OID est déjà attendu dans une colonne.
    for (i, slot) in active_columns.iter().enumerate() {
        if *slot == Some(oid) {
            return i;
        }
    }
    // Sinon, trouver la première colonne libre.
    assign_new_column(active_columns, oid)
}

/// Assigne un OID à la première colonne libre, ou en crée une nouvelle.
fn assign_new_column(active_columns: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    for (i, slot) in active_columns.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(oid);
            return i;
        }
    }
    active_columns.push(Some(oid));
    active_columns.len() - 1
}
