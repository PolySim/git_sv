use git2::{Oid, Repository};
use std::collections::HashMap;

use super::commit::CommitInfo;
use crate::error::Result;

/// Type de segment visuel dans le graphe.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    /// Ligne verticale continue (│).
    Vertical,
    /// Courbe vers la droite depuis le commit (╭─).
    ForkRight,
    /// Courbe vers la gauche depuis le commit (─╮).
    ForkLeft,
    /// Merge : courbe entrante depuis la droite (╰─).
    MergeFromRight,
    /// Merge : courbe entrante depuis la gauche (─╯).
    MergeFromLeft,
    /// Ligne horizontale de passage (─).
    Horizontal,
    /// Croisement de lignes (┼).
    Cross,
}

/// Cellule du graphe : représente ce qui est dessiné dans une colonne donnée.
#[derive(Debug, Clone)]
pub struct GraphCell {
    /// Type de segment à dessiner.
    pub edge_type: EdgeType,
    /// Index de couleur de la branche.
    pub color_index: usize,
}

/// Rangée intermédiaire entre deux commits (pour les connexions).
#[derive(Debug, Clone)]
pub struct ConnectionRow {
    /// Cellules de connexion pour chaque colonne.
    pub cells: Vec<Option<GraphCell>>,
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
    /// Cellules du graphe sur la ligne du commit (colonnes actives).
    pub cells: Vec<Option<GraphCell>>,
    /// Ligne de connexion vers la rangée suivante.
    pub connection: Option<ConnectionRow>,
}

/// État d'une colonne active pendant la construction du graphe.
#[derive(Debug, Clone)]
struct ColumnState {
    /// OID attendu dans cette colonne (le prochain commit qui doit l'utiliser).
    expected_oid: Option<Oid>,
    /// Index de couleur associé à cette colonne.
    color_index: usize,
}

/// Construit le graphe de commits avec placement en colonnes et edges de connexion.
pub fn build_graph(repo: &Repository, commits: &[CommitInfo]) -> Result<Vec<GraphRow>> {
    let mut rows = Vec::with_capacity(commits.len());

    // Colonnes actives : chaque slot contient l'état de la colonne (OID attendu + couleur).
    let mut active_columns: Vec<ColumnState> = Vec::new();

    // Mapping branche -> index de couleur stable.
    let mut branch_colors: HashMap<String, usize> = HashMap::new();
    let mut next_color_index: usize = 0;

    // Collecter les refs pour chaque OID.
    let refs_map = collect_refs(repo)?;

    for (commit_idx, ci) in commits.iter().enumerate() {
        let oid = ci.oid;

        // Trouver la colonne de ce commit.
        let column = find_or_assign_column(&mut active_columns, oid);

        // Récupérer les refs de ce commit.
        let refs = refs_map.get(&oid).cloned().unwrap_or_default();

        // Déterminer la couleur pour ce commit et cette colonne.
        let color_index = determine_color_index(
            column,
            &refs,
            &mut branch_colors,
            &mut next_color_index,
            &active_columns,
        );

        // Mettre à jour la couleur de la colonne.
        if column < active_columns.len() {
            active_columns[column].color_index = color_index;
        }

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

        // Générer les cellules pour la ligne du commit.
        let cells = build_commit_cells(column, &active_columns, color_index);

        // Libérer la colonne courante.
        if column < active_columns.len() {
            active_columns[column].expected_oid = None;
        }

        // Calculer les edges pour les parents et mettre à jour les colonnes.
        let parent_assignments =
            assign_parent_columns(&mut active_columns, column, ci, color_index);

        // Générer la ligne de connexion vers le commit suivant (s'il existe).
        let connection = if commit_idx + 1 < commits.len() {
            Some(build_connection_row(
                &active_columns,
                &parent_assignments,
                column,
            ))
        } else {
            None
        };

        rows.push(GraphRow {
            node,
            cells,
            connection,
        });
    }

    Ok(rows)
}

/// Construit les cellules pour la ligne du commit.
fn build_commit_cells(
    commit_col: usize,
    active_columns: &[ColumnState],
    _commit_color: usize,
) -> Vec<Option<GraphCell>> {
    let num_cols = active_columns.len().max(commit_col + 1);
    let mut cells: Vec<Option<GraphCell>> = Vec::with_capacity(num_cols);

    for col in 0..num_cols {
        if col == commit_col {
            // La colonne du commit elle-même - pas de ligne ici, juste le nœud.
            cells.push(None);
        } else if col < active_columns.len() && active_columns[col].expected_oid.is_some() {
            // Colonne active avec un flux de commits - ligne verticale.
            cells.push(Some(GraphCell {
                edge_type: EdgeType::Vertical,
                color_index: active_columns[col].color_index,
            }));
        } else {
            // Colonne inactive.
            cells.push(None);
        }
    }

    cells
}

/// Assigne les colonnes aux parents et retourne les assignations.
fn assign_parent_columns(
    active_columns: &mut Vec<ColumnState>,
    commit_col: usize,
    ci: &CommitInfo,
    commit_color: usize,
) -> Vec<(usize, usize, usize)> {
    // (parent_col, target_col, color_index)
    let mut assignments = Vec::new();

    for (i, &parent_oid) in ci.parents.iter().enumerate() {
        if i == 0 {
            // Premier parent : reste sur la même colonne.
            if commit_col < active_columns.len() {
                active_columns[commit_col].expected_oid = Some(parent_oid);
                // La couleur se propage au parent
                assignments.push((commit_col, commit_col, commit_color));
            } else {
                // Étendre si nécessaire
                while active_columns.len() <= commit_col {
                    active_columns.push(ColumnState {
                        expected_oid: None,
                        color_index: 0,
                    });
                }
                active_columns[commit_col].expected_oid = Some(parent_oid);
                active_columns[commit_col].color_index = commit_color;
                assignments.push((commit_col, commit_col, commit_color));
            }
        } else {
            // Parents supplémentaires (merge) : nouvelle colonne.
            let parent_col = assign_new_column(active_columns, parent_oid);
            // Les merges utilisent la couleur du commit source
            active_columns[parent_col].color_index = commit_color;
            assignments.push((commit_col, parent_col, commit_color));
        }
    }

    assignments
}

/// Construit la ligne de connexion entre le commit courant et le suivant.
fn build_connection_row(
    active_columns: &[ColumnState],
    parent_assignments: &[(usize, usize, usize)],
    _commit_col: usize,
) -> ConnectionRow {
    let num_cols = active_columns.len();
    let mut cells: Vec<Option<GraphCell>> = vec![None; num_cols];

    // D'abord, marquer toutes les colonnes actives avec des lignes verticales.
    for (col, state) in active_columns.iter().enumerate() {
        if state.expected_oid.is_some() {
            cells[col] = Some(GraphCell {
                edge_type: EdgeType::Vertical,
                color_index: state.color_index,
            });
        }
    }

    // Ensuite, traiter les edges spéciaux (forks et merges).
    for &(from_col, to_col, color) in parent_assignments {
        if from_col == to_col {
            // Edge vertical déjà traité ci-dessus.
            continue;
        }

        // C'est un merge ou un fork.
        if to_col > from_col {
            // Fork vers la droite ou merge depuis la droite.
            // from_col = commit, to_col = parent (merge depuis la droite)
            cells[from_col] = Some(GraphCell {
                edge_type: EdgeType::MergeFromRight,
                color_index: color,
            });

            // Lignes horizontales entre from_col et to_col
            for col in (from_col + 1)..to_col {
                cells[col] = Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: color,
                });
            }

            // Courbe d'entrée à to_col
            cells[to_col] = Some(GraphCell {
                edge_type: EdgeType::ForkRight,
                color_index: color,
            });
        } else {
            // Fork vers la gauche ou merge depuis la gauche.
            cells[from_col] = Some(GraphCell {
                edge_type: EdgeType::MergeFromLeft,
                color_index: color,
            });

            // Lignes horizontales entre to_col et from_col
            for col in (to_col + 1)..from_col {
                cells[col] = Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: color,
                });
            }

            // Courbe d'entrée à to_col
            cells[to_col] = Some(GraphCell {
                edge_type: EdgeType::ForkLeft,
                color_index: color,
            });
        }
    }

    ConnectionRow { cells }
}

/// Détermine l'index de couleur pour un commit.
fn determine_color_index(
    column: usize,
    refs: &[String],
    branch_colors: &mut HashMap<String, usize>,
    next_color_index: &mut usize,
    active_columns: &[ColumnState],
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

    // Sinon, utiliser la couleur de la colonne si elle existe.
    if column < active_columns.len() {
        // Si la colonne a déjà une couleur assignée, la réutiliser.
        if active_columns[column].color_index > 0 || column == 0 {
            return active_columns[column].color_index;
        }
    }

    // Fallback : utiliser l'index de colonne comme couleur.
    column
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
fn find_or_assign_column(active_columns: &mut Vec<ColumnState>, oid: Oid) -> usize {
    // Chercher si cet OID est déjà attendu dans une colonne.
    for (i, state) in active_columns.iter().enumerate() {
        if state.expected_oid == Some(oid) {
            return i;
        }
    }
    // Sinon, trouver la première colonne libre.
    assign_new_column(active_columns, oid)
}

/// Assigne un OID à la première colonne libre, ou en crée une nouvelle.
fn assign_new_column(active_columns: &mut Vec<ColumnState>, oid: Oid) -> usize {
    for (i, state) in active_columns.iter_mut().enumerate() {
        if state.expected_oid.is_none() {
            state.expected_oid = Some(oid);
            return i;
        }
    }
    // Aucune colonne libre, en créer une nouvelle.
    active_columns.push(ColumnState {
        expected_oid: Some(oid),
        color_index: 0,
    });
    active_columns.len() - 1
}
