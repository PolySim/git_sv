use git2::{Oid, Repository};
use std::collections::HashMap;

use super::commit::CommitInfo;
use crate::error::Result;

/// Type de référence git.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefType {
    /// Branche locale.
    LocalBranch,
    /// Branche remote (origin/main, etc.).
    RemoteBranch,
    /// Tag.
    Tag,
    /// HEAD détaché ou HEAD pointant vers cette branche.
    Head,
}

/// Information sur une référence git.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefInfo {
    /// Nom de la référence.
    pub name: String,
    /// Type de la référence.
    pub ref_type: RefType,
}

impl RefInfo {
    /// Crée une nouvelle RefInfo.
    pub fn new(name: impl Into<String>, ref_type: RefType) -> Self {
        Self {
            name: name.into(),
            ref_type,
        }
    }
}

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
    /// Références pointant vers ce commit (branches, tags).
    pub refs: Vec<RefInfo>,
    /// Nom de la branche à laquelle appartient ce commit dans le graphe.
    pub branch_name: Option<String>,
    /// Colonne assignée pour le rendu du graphe.
    pub column: usize,
    /// Index de couleur stable pour cette branche.
    pub color_index: usize,
}

impl Default for CommitNode {
    fn default() -> Self {
        Self {
            oid: Oid::zero(),
            message: String::new(),
            author: String::new(),
            timestamp: 0,
            parents: Vec::new(),
            refs: Vec::new(),
            branch_name: None,
            column: 0,
            color_index: 0,
        }
    }
}

impl CommitNode {
    /// Retourne le hash court du commit (7 premiers caractères).
    pub fn short_hash(&self) -> String {
        self.oid.to_string()[..7].to_string()
    }

    /// Vérifie si ce commit est HEAD.
    pub fn is_head(&self) -> bool {
        self.refs.iter().any(|r| r.ref_type == RefType::Head)
    }

    /// Retourne les branches locales de ce commit.
    pub fn local_branches(&self) -> impl Iterator<Item = &str> {
        self.refs
            .iter()
            .filter(|r| r.ref_type == RefType::LocalBranch)
            .map(|r| r.name.as_str())
    }

    /// Retourne les tags de ce commit.
    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.refs
            .iter()
            .filter(|r| r.ref_type == RefType::Tag)
            .map(|r| r.name.as_str())
    }
}

/// Rangée du graphe contenant le commit et les segments de connexion.
#[derive(Debug, Clone, Default)]
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
    /// Nom de la branche associée à cette colonne.
    branch_name: Option<String>,
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

        // Déterminer le nom de la branche pour ce commit.
        let branch_name = determine_branch_name(column, &refs, &active_columns);

        // Mettre à jour la couleur et le nom de la branche de la colonne.
        if column < active_columns.len() {
            active_columns[column].color_index = color_index;
            if active_columns[column].branch_name.is_none() && branch_name.is_some() {
                active_columns[column].branch_name = branch_name.clone();
            }
        }

        // Créer le noeud.
        let node = CommitNode {
            oid,
            message: ci.message.clone(),
            author: ci.author.clone(),
            timestamp: ci.timestamp,
            parents: ci.parents.clone(),
            refs,
            branch_name: branch_name.clone(),
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

        // Compacter : supprimer les colonnes terminales vides (celles sans expected_oid).
        // On ne supprime que par la droite pour maintenir l'alignement des colonnes internes.
        while active_columns.last().map_or(false, |s| s.expected_oid.is_none()) {
            active_columns.pop();
        }

        // Générer la ligne de connexion vers le commit suivant (s'il existe).
        // La connexion est générée APRÈS compaction pour refléter l'état compacté.
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
                        branch_name: None,
                    });
                }
                active_columns[commit_col].expected_oid = Some(parent_oid);
                active_columns[commit_col].color_index = commit_color;
                assignments.push((commit_col, commit_col, commit_color));
            }
        } else {
            // Parents supplémentaires (merge).
            // Chercher si ce parent est déjà dans une colonne active.
            let existing_col = active_columns
                .iter()
                .position(|s| s.expected_oid == Some(parent_oid));

            if let Some(parent_col) = existing_col {
                // Le parent est déjà suivi dans une colonne existante.
                // On ne modifie PAS expected_oid ni color_index de cette colonne.
                // On utilise la couleur de la colonne cible pour le lien de merge.
                let merge_color = active_columns[parent_col].color_index;
                assignments.push((commit_col, parent_col, merge_color));
            } else {
                // Parent pas encore dans le graphe → nouvelle colonne.
                let parent_col = assign_new_column(active_columns, parent_oid);
                active_columns[parent_col].color_index = commit_color;
                assignments.push((commit_col, parent_col, commit_color));
            }
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

    // Supprimer les lignes verticales aux positions d'arrivée des merges/forks
    // pour éviter qu'elles traversent les points de connexion.
    for &(from_col, to_col, _color) in parent_assignments {
        if from_col != to_col && to_col < cells.len() {
            cells[to_col] = None;
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
                if cells[col]
                    .as_ref()
                    .map_or(false, |c| c.edge_type == EdgeType::Vertical)
                {
                    // Croisement avec une ligne verticale existante.
                    let existing_color = cells[col].as_ref().unwrap().color_index;
                    cells[col] = Some(GraphCell {
                        edge_type: EdgeType::Cross,
                        color_index: existing_color,
                    });
                } else {
                    cells[col] = Some(GraphCell {
                        edge_type: EdgeType::Horizontal,
                        color_index: color,
                    });
                }
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
                if cells[col]
                    .as_ref()
                    .map_or(false, |c| c.edge_type == EdgeType::Vertical)
                {
                    // Croisement avec une ligne verticale existante.
                    let existing_color = cells[col].as_ref().unwrap().color_index;
                    cells[col] = Some(GraphCell {
                        edge_type: EdgeType::Cross,
                        color_index: existing_color,
                    });
                } else {
                    cells[col] = Some(GraphCell {
                        edge_type: EdgeType::Horizontal,
                        color_index: color,
                    });
                }
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
    refs: &[RefInfo],
    branch_colors: &mut HashMap<String, usize>,
    next_color_index: &mut usize,
    active_columns: &[ColumnState],
) -> usize {
    // Si le commit a des refs (branches/tags), utiliser la première comme couleur.
    if let Some(first_ref) = refs.first() {
        if let Some(&color) = branch_colors.get(&first_ref.name) {
            return color;
        }
        // Nouvelle branche : assigner une nouvelle couleur.
        let color = *next_color_index;
        branch_colors.insert(first_ref.name.clone(), color);
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

/// Détermine le nom de la branche pour un commit.
fn determine_branch_name(
    column: usize,
    refs: &[RefInfo],
    active_columns: &[ColumnState],
) -> Option<String> {
    // Si le commit a des refs, utiliser la première comme nom de branche.
    // On filtre pour ne garder que les branches (pas les tags).
    if let Some(first_ref) = refs.first() {
        let name = &first_ref.name;
        if name.starts_with("refs/heads/") {
            return Some(name.strip_prefix("refs/heads/").unwrap().to_string());
        }
        if !name.contains('/') {
            return Some(name.clone());
        }
        return Some(name.clone());
    }

    // Sinon, hériter du nom de la colonne si elle existe.
    if column < active_columns.len() {
        return active_columns[column].branch_name.clone();
    }

    None
}

/// Collecte toutes les références (branches, tags) et les associe à leur OID avec leur type.
fn collect_refs(repo: &Repository) -> Result<HashMap<Oid, Vec<RefInfo>>> {
    let mut map: HashMap<Oid, Vec<RefInfo>> = HashMap::new();

    // Déterminer HEAD
    let head_oid = repo.head().ok().and_then(|h| h.target());
    let head_branch = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            h.shorthand().map(|s| s.to_string())
        } else {
            None
        }
    });

    for reference in repo.references()? {
        let reference = reference?;
        if let Some(name) = reference.shorthand() {
            // Ignorer HEAD directement (on le gère via head_branch)
            if name == "HEAD" {
                continue;
            }

            let ref_type = if reference.is_tag() {
                RefType::Tag
            } else if reference.is_remote() || name.contains('/') {
                RefType::RemoteBranch
            } else {
                RefType::LocalBranch
            };

            // Pour les tags, récupérer l'OID du commit pointé (pas l'OID du tag lui-même)
            let target_oid = if reference.is_tag() {
                reference
                    .peel(git2::ObjectType::Commit)
                    .ok()
                    .and_then(|obj| obj.as_commit().map(|c| c.id()))
            } else {
                reference.target()
            };

            if let Some(oid) = target_oid {
                map.entry(oid)
                    .or_default()
                    .push(RefInfo {
                        name: name.to_string(),
                        ref_type,
                    });
            }
        }
    }

    // Marquer HEAD
    if let (Some(oid), Some(branch)) = (head_oid, head_branch) {
        if let Some(refs) = map.get_mut(&oid) {
            for r in refs.iter_mut() {
                if r.name == branch {
                    r.ref_type = RefType::Head;
                }
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
        branch_name: None,
    });
    active_columns.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

    #[test]
    fn test_build_graph_linear() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer une histoire linéaire: A -> B -> C
        let oid_a = commit_file(&repo, "file.txt", "A", "First commit");
        let oid_b = commit_file(&repo, "file.txt", "B", "Second commit");
        let oid_c = commit_file(&repo, "file.txt", "C", "Third commit");

        // Récupérer les commits
        let commits = vec![
            CommitInfo::from_git2_commit(&repo.find_commit(oid_c).unwrap()),
            CommitInfo::from_git2_commit(&repo.find_commit(oid_b).unwrap()),
            CommitInfo::from_git2_commit(&repo.find_commit(oid_a).unwrap()),
        ];

        // Construire le graphe
        let graph = build_graph(&repo, &commits).unwrap();

        // Devrait avoir 3 rangées
        assert_eq!(graph.len(), 3);

        // Tous les commits devraient être sur la même colonne (0)
        assert_eq!(graph[0].node.column, 0);
        assert_eq!(graph[1].node.column, 0);
        assert_eq!(graph[2].node.column, 0);
    }

    #[test]
    fn test_find_or_assign_column() {
        let mut columns: Vec<ColumnState> = vec![];
        let oid1 = Oid::from_bytes(&[1; 20]).unwrap();
        let oid2 = Oid::from_bytes(&[2; 20]).unwrap();

        // Premier OID : nouvelle colonne
        let col1 = find_or_assign_column(&mut columns, oid1);
        assert_eq!(col1, 0);
        assert_eq!(columns.len(), 1);

        // Deuxième OID différent : nouvelle colonne
        let col2 = find_or_assign_column(&mut columns, oid2);
        assert_eq!(col2, 1);
        assert_eq!(columns.len(), 2);

        // Même OID que le premier : colonne existante
        let col1_again = find_or_assign_column(&mut columns, oid1);
        assert_eq!(col1_again, 0);
    }

    #[test]
    fn test_assign_new_column_reuse() {
        let mut columns: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None,
                color_index: 0,
                branch_name: None,
            }, // Libre
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
        ];

        let new_oid = Oid::from_bytes(&[3; 20]).unwrap();
        let col = assign_new_column(&mut columns, new_oid);

        // Devrait réutiliser la colonne 1 (la libre)
        assert_eq!(col, 1);
        assert_eq!(columns[1].expected_oid, Some(new_oid));
    }

    #[test]
    fn test_determine_color_index() {
        let mut branch_colors: HashMap<String, usize> = HashMap::new();
        let mut next_color: usize = 0;
        let columns: Vec<ColumnState> = vec![];

        // Sans refs, utilise l'index de colonne
        let color = determine_color_index(0, &[], &mut branch_colors, &mut next_color, &columns);
        assert_eq!(color, 0);

        // Avec refs, assigne une nouvelle couleur
        let color2 = determine_color_index(
            0,
            &vec![RefInfo::new("feature", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color2, 0);
        assert_eq!(next_color, 1);

        // Même ref, même couleur
        let color3 = determine_color_index(
            0,
            &vec![RefInfo::new("feature", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color3, 0);

        // Nouvelle ref, nouvelle couleur
        let color4 = determine_color_index(
            0,
            &vec![RefInfo::new("main", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color4, 1);
        assert_eq!(next_color, 2);
    }

    #[test]
    fn test_collect_refs() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        let oid = commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche
        let commit = repo.find_commit(oid).unwrap();
        repo.branch("feature", &commit, false).unwrap();

        // Collecter les refs
        let refs_map = collect_refs(&repo).unwrap();

        // Devrait avoir des entrées
        assert!(!refs_map.is_empty());

        // La branche main et feature devraient pointer vers le commit
        if let Some(refs) = refs_map.get(&oid) {
            assert!(refs.iter().any(|r| r.name.contains("main")));
            assert!(refs.iter().any(|r| r.name.contains("feature")));
        } else {
            panic!("Commit non trouvé dans refs_map");
        }
    }

    #[test]
    fn test_column_compaction() {
        // Test que les colonnes terminales vides sont compactées
        use git2::Oid;

        // Simuler un scénario avec 3 colonnes, où la colonne 2 devient vide
        let mut active_columns: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 1,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None, // Cette colonne est vide
                color_index: 0,
                branch_name: None,
            },
        ];

        // Compacter
        while active_columns.last().map_or(false, |s| s.expected_oid.is_none()) {
            active_columns.pop();
        }

        // La colonne vide en fin devrait être supprimée
        assert_eq!(active_columns.len(), 2, "La colonne vide terminale devrait être supprimée");
        assert!(active_columns[0].expected_oid.is_some());
        assert!(active_columns[1].expected_oid.is_some());

        // Test avec colonnes vides au milieu (ne doivent PAS être supprimées)
        let mut active_columns2: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None, // Vide au milieu
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 1,
                branch_name: None,
            },
        ];

        // Compacter
        while active_columns2.last().map_or(false, |s| s.expected_oid.is_none()) {
            active_columns2.pop();
        }

        // Seule la dernière colonne est supprimée si vide, pas celle du milieu
        assert_eq!(active_columns2.len(), 3, "Les colonnes vides au milieu ne devraient pas être supprimées");
    }

    #[test]
    fn test_ref_classification() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        let oid = commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche feature
        let commit = repo.find_commit(oid).unwrap();
        repo.branch("feature", &commit, false).unwrap();

        // Créer un tag v1.0
        repo.tag(
            "v1.0",
            &commit.clone().into_object(),
            &git2::Signature::now("Test", "test@test.com").unwrap(),
            "Version 1.0",
            false,
        )
        .unwrap();

        // Collecter les refs
        let refs_map = collect_refs(&repo).unwrap();
        let commit_refs = refs_map.get(&oid).expect("Le commit devrait avoir des refs");

        // Vérifier les types de refs
        assert!(
            commit_refs.iter().any(|r| r.ref_type == RefType::Head && r.name == "main"),
            "Devrait avoir HEAD sur main"
        );
        assert!(
            commit_refs.iter().any(|r| r.ref_type == RefType::LocalBranch && r.name == "feature"),
            "Devrait avoir une branche locale 'feature'"
        );
        assert!(
            commit_refs.iter().any(|r| r.ref_type == RefType::Tag && r.name == "v1.0"),
            "Devrait avoir un tag 'v1.0'"
        );
    }
}
