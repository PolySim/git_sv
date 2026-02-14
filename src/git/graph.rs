use git2::{Oid, Repository};
use std::collections::HashMap;

use super::commit::CommitInfo;
use crate::error::Result;

/// Segment de ligne à dessiner entre deux rangées de commits.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Colonne de départ (rangée du dessus).
    pub from_col: usize,
    /// Colonne d'arrivée (rangée du dessous).
    pub to_col: usize,
    /// Index de couleur associé à cette branche.
    pub color_index: usize,
}

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
    /// Index de couleur stable pour cette branche.
    pub color_index: usize,
}

/// Rangée du graphe contenant le commit et les segments de connexion.
#[derive(Debug, Clone)]
pub struct GraphRow {
    /// Le commit de cette rangée.
    pub node: CommitNode,
    /// Les segments de lignes à dessiner sur cette rangée (entre ce commit et le suivant).
    pub edges: Vec<Edge>,
}

/// Construit le graphe de commits avec placement en colonnes et edges de connexion.
pub fn build_graph(repo: &Repository, commits: &[CommitInfo]) -> Result<Vec<GraphRow>> {
    let mut rows = Vec::with_capacity(commits.len());

    // Colonnes actives : chaque slot contient l'OID attendu dans cette colonne.
    let mut active_columns: Vec<Option<Oid>> = Vec::new();

    // Mapping branche -> index de couleur stable.
    let mut branch_colors: HashMap<String, usize> = HashMap::new();
    let mut next_color_index: usize = 0;

    // Collecter les refs pour chaque OID.
    let refs_map = collect_refs(repo)?;

    for ci in commits {
        let oid = ci.oid;

        // Trouver la colonne de ce commit.
        let column = find_or_assign_column(&mut active_columns, oid);

        // Récupérer les refs de ce commit et assigner des couleurs.
        let refs = refs_map.get(&oid).cloned().unwrap_or_default();
        let color_index = determine_color_index(
            column,
            &refs,
            &mut branch_colors,
            &mut next_color_index,
            &active_columns,
        );

        // Créer le noeud.
        let node = CommitNode {
            oid,
            message: ci.message.clone(),
            author: ci.author.clone(),
            timestamp: ci.timestamp,
            parents: ci.parents.clone(),
            refs,
            column,
            color_index,
        };

        // Libérer la colonne courante.
        if column < active_columns.len() {
            active_columns[column] = None;
        }

        // Calculer les edges pour les parents.
        let mut edges = Vec::new();

        for (i, &parent_oid) in ci.parents.iter().enumerate() {
            if i == 0 {
                // Premier parent : edge vertical sur la même colonne.
                edges.push(Edge {
                    from_col: column,
                    to_col: column,
                    color_index,
                });

                // Assigner le parent à la même colonne.
                if column < active_columns.len() {
                    active_columns[column] = Some(parent_oid);
                } else {
                    while active_columns.len() <= column {
                        active_columns.push(None);
                    }
                    active_columns[column] = Some(parent_oid);
                }
            } else {
                // Parents supplémentaires (merge) : edge diagonal vers nouvelle colonne.
                let parent_column = assign_new_column(&mut active_columns, parent_oid);
                edges.push(Edge {
                    from_col: column,
                    to_col: parent_column,
                    color_index,
                });
            }
        }

        // Ajouter les edges verticaux pour les colonnes qui traversent.
        let num_columns = active_columns.len();
        for col in 0..num_columns {
            if col != column && active_columns[col].is_some() {
                // Trouver la couleur de cette colonne.
                let col_color = find_column_color(col, &rows);
                edges.push(Edge {
                    from_col: col,
                    to_col: col,
                    color_index: col_color,
                });
            }
        }

        rows.push(GraphRow { node, edges });
    }

    Ok(rows)
}

/// Détermine l'index de couleur pour un commit.
fn determine_color_index(
    column: usize,
    refs: &[String],
    branch_colors: &mut HashMap<String, usize>,
    next_color_index: &mut usize,
    _active_columns: &[Option<Oid>],
) -> usize {
    // Si le commit a des refs (branches/tags), utiliser la première comme couleur.
    if let Some(first_ref) = refs.first() {
        if let Some(&color) = branch_colors.get(first_ref) {
            return color;
        }
        // Nouvelle branche : assigner une nouvelle couleur.
        let color = *next_color_index;
        branch_colors.insert(first_ref.clone(), color);
        *next_color_index += 1;
        return color;
    }

    // Sinon, utiliser la couleur de la colonne.
    column
}

/// Trouve la couleur associée à une colonne en cherchant dans les rangées précédentes.
fn find_column_color(col: usize, rows: &[GraphRow]) -> usize {
    // Chercher en arrière pour trouver un edge sur cette colonne.
    for row in rows.iter().rev() {
        for edge in &row.edges {
            if edge.from_col == col || edge.to_col == col {
                return edge.color_index;
            }
        }
    }
    col // Fallback : utiliser l'index de colonne comme couleur.
}

/// Collecte toutes les références (branches, tags) et les associe à leur OID.
fn collect_refs(repo: &Repository) -> Result<HashMap<Oid, Vec<String>>> {
    let mut map = HashMap::new();

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
