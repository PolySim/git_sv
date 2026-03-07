//! État de la vue graph.

#![allow(dead_code)]

use crate::git::diff::DiffViewMode;
use crate::git::diff::{DiffFile, FileDiff};
use crate::git::graph::GraphRow;
use crate::state::selection::ListSelection;
use ratatui::widgets::ListState;

/// Nombre d'items visuels par commit (1 ligne commit + 1 ligne connexion).
const ITEMS_PER_COMMIT: usize = 2;

/// État unifié de la vue graph.
///
/// Cette structure est la source unique de vérité pour :
/// - La liste des commits affichés
/// - La sélection du commit
/// - La sélection du fichier dans le commit
/// - Les offsets de scroll associés
/// - Le diff du fichier sélectionné
/// - La pagination de l'historique
#[derive(Debug, Clone)]
pub struct GraphViewState {
    /// Lignes du graph avec sélection.
    pub rows: ListSelection<GraphRow>,
    /// État de la liste ratatui (pour le rendu).
    pub list_state: ListState,
    /// Index du fichier sélectionné dans le commit courant.
    pub file_selected_index: usize,
    /// Fichiers du commit sélectionné.
    pub commit_files: Vec<DiffFile>,
    /// Diff du fichier sélectionné.
    pub selected_file_diff: Option<FileDiff>,
    /// Offset de scroll vertical dans le diff.
    pub diff_scroll_offset: usize,
    /// Offset de scroll horizontal dans le diff.
    pub diff_horizontal_offset: usize,
    /// Nombre total de lignes dans le diff actuel.
    pub diff_total_lines: usize,
    /// Mode plein écran pour le diff.
    pub diff_fullscreen: bool,
    /// Mode d'affichage du diff (unifié ou côte à côte).
    pub diff_view_mode: DiffViewMode,
    /// Nombre de commits actuellement chargés.
    pub loaded_count: usize,
    /// Nombre total estimé de commits dans le repo (si connu).
    pub total_commits: Option<usize>,
    /// Indique si plus d'historique peut être chargé.
    pub can_load_more: bool,
    /// Indique si un chargement est en cours.
    pub is_loading_more: bool,
}

impl Default for GraphViewState {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            rows: ListSelection::new(),
            list_state,
            file_selected_index: 0,
            commit_files: Vec::new(),
            selected_file_diff: None,
            diff_scroll_offset: 0,
            diff_horizontal_offset: 0,
            diff_total_lines: 0,
            diff_fullscreen: false,
            diff_view_mode: DiffViewMode::default(),
            loaded_count: 0,
            total_commits: None,
            can_load_more: true,
            is_loading_more: false,
        }
    }
}

impl GraphViewState {
    /// Crée un nouvel état graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retourne l'index du commit sélectionné.
    pub fn selected_index(&self) -> usize {
        self.rows.selected_index()
    }

    /// Retourne le commit actuellement sélectionné.
    pub fn selected_commit(&self) -> Option<&crate::git::graph::CommitNode> {
        self.rows.selected_item().map(|row| &row.node)
    }

    /// Retourne le nombre de commits dans le graphe.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Le graphe est-il vide ?
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Calcule l'index visuel pour ratatui à partir de l'index du commit.
    ///
    /// Chaque commit produit 1 item + potentiellement 1 item de connexion.
    /// Cette fonction retourne l'index visuel dans la liste ratatui.
    pub fn visual_index(&self) -> usize {
        if self.rows.is_empty() {
            0
        } else {
            self.rows.selected_index() * ITEMS_PER_COMMIT
        }
    }

    /// Remplace le graphe par un nouveau et ajuste la sélection si nécessaire.
    ///
    /// Si le commit sélectionné existe encore dans le nouveau graphe, conserve la sélection.
    /// Sinon, ajuste l'index pour rester dans les bornes.
    pub fn replace_graph(&mut self, new_graph: Vec<GraphRow>) {
        // Sauvegarder l'oid du commit actuellement sélectionné
        let current_oid = self.selected_commit().map(|node| node.oid);

        // Remplacer les items
        self.rows.set_items(new_graph);

        // Chercher si le commit existe encore
        if let Some(oid) = current_oid {
            if let Some(new_index) = self.rows.items.iter().position(|row| row.node.oid == oid) {
                // Le commit existe encore, sélectionner son nouvel index
                self.rows.select(new_index);
            } else {
                // Le commit n'existe plus, ajuster l'index
                self.clamp_selection();
            }
        } else {
            // Pas de commit sélectionné, rester à 0
            self.rows.select(0);
        }

        // Synchroniser l'état ratatui
        self.sync_list_state();
    }

    /// Sélectionne un commit par son index.
    ///
    /// # Panics
    /// Panique si l'index est hors des bornes (en mode debug).
    pub fn select_commit(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.select(index);
            self.sync_list_state();
            // Réinitialiser la sélection de fichier
            self.file_selected_index = 0;
        }
    }

    /// Sélectionne le commit précédent.
    pub fn select_previous(&mut self) {
        self.rows.select_previous();
        self.sync_list_state();
        self.file_selected_index = 0;
    }

    /// Sélectionne le commit suivant.
    pub fn select_next(&mut self) {
        self.rows.select_next();
        self.sync_list_state();
        self.file_selected_index = 0;
    }

    /// Remonte d'une page.
    pub fn page_up(&mut self) {
        self.rows.page_up();
        self.sync_list_state();
    }

    /// Descend d'une page.
    pub fn page_down(&mut self) {
        self.rows.page_down();
        self.sync_list_state();
    }

    /// Va au premier commit.
    pub fn go_top(&mut self) {
        self.rows.select_first();
        self.sync_list_state();
    }

    /// Va au dernier commit.
    pub fn go_bottom(&mut self) {
        self.rows.select_last();
        self.sync_list_state();
    }

    /// Sélectionne un fichier par son index.
    pub fn select_file(&mut self, index: usize) {
        if index < self.commit_files.len() {
            self.file_selected_index = index;
        }
    }

    /// Sélectionne le fichier précédent.
    pub fn select_previous_file(&mut self) {
        if self.file_selected_index > 0 {
            self.file_selected_index -= 1;
        }
    }

    /// Sélectionne le fichier suivant.
    pub fn select_next_file(&mut self) {
        if self.file_selected_index + 1 < self.commit_files.len() {
            self.file_selected_index += 1;
        }
    }

    /// Définit les fichiers du commit sélectionné et ajuste la sélection de fichier.
    pub fn set_commit_files(&mut self, files: Vec<DiffFile>) {
        self.commit_files = files;
        // S'assurer que l'index du fichier est dans les bornes
        if self.file_selected_index >= self.commit_files.len() && !self.commit_files.is_empty() {
            self.file_selected_index = self.commit_files.len() - 1;
        } else if self.commit_files.is_empty() {
            self.file_selected_index = 0;
        }
    }

    /// Définit le diff du fichier sélectionné et réinitialise les scrolls.
    pub fn set_file_diff(&mut self, diff: Option<FileDiff>) {
        self.selected_file_diff = diff;
        // Réinitialiser tous les offsets de scroll
        self.diff_scroll_offset = 0;
        self.diff_horizontal_offset = 0;
        self.diff_total_lines = 0;
    }

    /// Efface le diff sélectionné.
    pub fn clear_file_diff(&mut self) {
        self.selected_file_diff = None;
        self.diff_scroll_offset = 0;
        self.diff_horizontal_offset = 0;
        self.diff_total_lines = 0;
    }

    /// Scroll le diff vers le haut.
    pub fn scroll_diff_up(&mut self) {
        if self.diff_scroll_offset > 0 {
            self.diff_scroll_offset -= 1;
        }
    }

    /// Scroll le diff vers le bas.
    pub fn scroll_diff_down(&mut self) {
        let max_scroll = self.diff_total_lines.saturating_sub(20); // 20 = hauteur estimée
        if self.diff_scroll_offset < max_scroll {
            self.diff_scroll_offset += 1;
        }
    }

    /// Scroll le diff d'une page vers le haut.
    pub fn scroll_diff_page_up(&mut self) {
        let page_size = 10;
        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(page_size);
    }

    /// Scroll le diff d'une page vers le bas.
    pub fn scroll_diff_page_down(&mut self) {
        let page_size = 10;
        let max_scroll = self.diff_total_lines.saturating_sub(20);
        self.diff_scroll_offset = (self.diff_scroll_offset + page_size).min(max_scroll);
    }

    /// Scroll le diff tout en haut.
    pub fn scroll_diff_top(&mut self) {
        self.diff_scroll_offset = 0;
    }

    /// Scroll le diff tout en bas.
    pub fn scroll_diff_bottom(&mut self) {
        let max_scroll = self.diff_total_lines.saturating_sub(20);
        self.diff_scroll_offset = max_scroll;
    }

    /// Scroll le diff vers la gauche.
    pub fn scroll_diff_left(&mut self) {
        if self.diff_horizontal_offset > 0 {
            self.diff_horizontal_offset -= 1;
        }
    }

    /// Scroll le diff vers la droite.
    pub fn scroll_diff_right(&mut self) {
        self.diff_horizontal_offset += 1;
    }

    /// Bascule le mode plein écran du diff.
    pub fn toggle_diff_fullscreen(&mut self) {
        self.diff_fullscreen = !self.diff_fullscreen;
    }

    /// Bascule le mode d'affichage du diff.
    pub fn toggle_diff_view_mode(&mut self) {
        self.diff_view_mode = match self.diff_view_mode {
            DiffViewMode::Unified => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Unified,
        };
    }

    /// Synchronise l'état de la liste ratatui avec la sélection actuelle.
    fn sync_list_state(&mut self) {
        let visual_idx = self.visual_index();
        self.list_state.select(Some(visual_idx));
    }

    /// Ajuste la sélection pour rester dans les bornes du graphe.
    fn clamp_selection(&mut self) {
        if self.rows.is_empty() {
            self.rows.select(0);
        } else if self.rows.selected_index() >= self.rows.len() {
            self.rows.select(self.rows.len() - 1);
        }
    }

    // ═══════════════════════════════════════════════════
    // Pagination de l'historique
    // ═══════════════════════════════════════════════════

    /// Met à jour l'état de pagination après un chargement.
    pub fn update_pagination_state(&mut self, loaded_count: usize, total_commits: Option<usize>) {
        self.loaded_count = loaded_count;
        self.total_commits = total_commits;

        // Déterminer si on peut charger plus
        if let Some(total) = total_commits {
            self.can_load_more =
                loaded_count < total && loaded_count < crate::state::MAX_TOTAL_COMMITS;
        } else {
            // Si on ne connaît pas le total, on suppose qu'on peut charger plus
            // tant qu'on n'a pas atteint la limite de sécurité
            self.can_load_more = loaded_count < crate::state::MAX_TOTAL_COMMITS;
        }
    }

    /// Ajoute des commits au graphe existant (pour le chargement progressif).
    ///
    /// Cette méthode étend le graphe actuel avec de nouveaux commits,
    /// en préservant la sélection de l'utilisateur.
    pub fn append_commits(&mut self, additional_rows: Vec<GraphRow>) {
        if additional_rows.is_empty() {
            // Plus rien à charger
            self.can_load_more = false;
            return;
        }

        // Sauvegarder l'OID du commit actuellement sélectionné
        let current_oid = self.selected_commit().map(|node| node.oid);
        let current_index = self.selected_index();

        // Étendre le graphe avec les nouveaux commits
        self.rows.items.extend(additional_rows);

        // Restaurer la sélection (l'index devrait être le même car on ajoute à la fin)
        if let Some(oid) = current_oid {
            if let Some(new_index) = self.rows.items.iter().position(|row| row.node.oid == oid) {
                self.rows.select(new_index);
            } else {
                // Le commit n'a pas été trouvé, utiliser l'ancien index
                self.rows
                    .select(current_index.min(self.rows.len().saturating_sub(1)));
            }
        }

        // Mettre à jour le compteur
        self.loaded_count = self.rows.len();
        self.sync_list_state();
    }

    /// Marque le début d'un chargement de commits supplémentaires.
    pub fn start_loading_more(&mut self) {
        self.is_loading_more = true;
    }

    /// Marque la fin d'un chargement de commits supplémentaires.
    pub fn finish_loading_more(&mut self) {
        self.is_loading_more = false;
    }

    /// Retourne le nombre de commits à charger pour la prochaine page.
    pub fn next_batch_size(&self) -> usize {
        crate::state::COMMIT_BATCH_SIZE
    }

    /// Retourne le nombre total de commits à charger (actuel + prochain lot).
    pub fn target_count_for_next_load(&self) -> usize {
        (self.loaded_count + self.next_batch_size()).min(crate::state::MAX_TOTAL_COMMITS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, GraphRow};
    use git2::Oid;

    fn create_test_graph(size: usize) -> Vec<GraphRow> {
        (0..size)
            .map(|i| GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[i as u8; 20]).unwrap_or(Oid::zero()),
                    message: format!("Commit {}", i),
                    author: "Test".to_string(),
                    timestamp: i as i64 * 1000,
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![None],
                connection: if i + 1 < size {
                    Some(crate::git::graph::ConnectionRow { cells: vec![] })
                } else {
                    None
                },
            })
            .collect()
    }

    #[test]
    fn test_visual_index() {
        let mut state = GraphViewState::new();
        state.rows = ListSelection::with_items(create_test_graph(5));
        state.rows.select(2);

        // Index visuel = 2 * 2 = 4
        assert_eq!(state.visual_index(), 4);
    }

    #[test]
    fn test_visual_index_empty() {
        let state = GraphViewState::new();
        assert_eq!(state.visual_index(), 0);
    }

    #[test]
    fn test_replace_graph_preserves_selection() {
        let mut state = GraphViewState::new();
        let graph1 = create_test_graph(5);
        state.rows = ListSelection::with_items(graph1);
        state.rows.select(2);

        // Remplacer par un graphe avec le même commit à l'index 3
        let mut graph2 = create_test_graph(5);
        // Déplacer le commit 2 à l'index 3
        let row = graph2.remove(2);
        graph2.insert(3, row);

        state.replace_graph(graph2);

        // La sélection devrait avoir suivi le commit
        assert_eq!(state.selected_index(), 3);
    }

    #[test]
    fn test_replace_graph_clamps_when_commit_removed() {
        let mut state = GraphViewState::new();
        state.rows = ListSelection::with_items(create_test_graph(5));
        state.rows.select(4); // Dernier commit

        // Remplacer par un graphe plus petit
        state.replace_graph(create_test_graph(3));

        // La sélection devrait être clampée
        assert_eq!(state.selected_index(), 2);
    }

    #[test]
    fn test_select_commit_bounds() {
        let mut state = GraphViewState::new();
        state.rows = ListSelection::with_items(create_test_graph(5));

        // Sélection valide
        state.select_commit(3);
        assert_eq!(state.selected_index(), 3);

        // Sélection hors bornes (ignorée)
        state.select_commit(10);
        assert_eq!(state.selected_index(), 3); // Inchangé
    }

    #[test]
    fn test_set_commit_files_adjusts_index() {
        let mut state = GraphViewState::new();
        state.file_selected_index = 5;

        // Définir moins de fichiers que l'index
        let files = vec![
            DiffFile {
                path: "a.txt".to_string(),
                old_path: None,
                status: crate::git::diff::DiffStatus::Added,
                additions: 1,
                deletions: 0,
            },
            DiffFile {
                path: "b.txt".to_string(),
                old_path: None,
                status: crate::git::diff::DiffStatus::Modified,
                additions: 0,
                deletions: 1,
            },
        ];
        state.set_commit_files(files);

        // L'index devrait être ajusté
        assert_eq!(state.file_selected_index, 1);
    }
}
