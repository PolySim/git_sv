//! État global de l'application.

pub mod action;
pub mod cache;
pub mod filter;
pub mod selection;
pub mod view;

pub use action::AppAction;
pub use cache::{DiffCache, DiffCacheKey, LazyBlame, LazyDiff};
pub use filter::{FilterField, FilterPopupState, GraphFilter};
pub use selection::ListSelection;
pub use view::*;

use crate::git::branch::BranchInfo;
use crate::git::diff::{DiffFile, DiffViewMode};
use crate::git::graph::GraphRow;
use crate::git::repo::{GitRepo, StatusEntry};
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

/// Nombre maximum de commits à charger.
pub const MAX_COMMITS: usize = 200;

/// État principal de l'application.
pub struct AppState {
    // ═══════════════════════════════════════════════════
    // Core
    // ═══════════════════════════════════════════════════
    /// Repository git.
    pub repo: GitRepo,

    /// Chemin du repository.
    pub repo_path: String,

    /// Branche courante.
    pub current_branch: Option<String>,

    // ═══════════════════════════════════════════════════
    // Vue active
    // ═══════════════════════════════════════════════════
    /// Mode de vue actuel.
    pub view_mode: ViewMode,

    /// Mode de vue précédent (pour retour depuis Help).
    pub previous_view_mode: Option<ViewMode>,

    /// État indiquant si un refresh est nécessaire.
    pub dirty: bool,

    // ═══════════════════════════════════════════════════
    // Vue Graph (toujours chargée)
    // ═══════════════════════════════════════════════════
    /// Lignes du graph de commits (compatibilité - migrer vers graph_view.rows).
    pub graph: Vec<GraphRow>,

    /// État de la vue graph avec sélection générique.
    pub graph_view: GraphViewState,

    /// Mode d'affichage du panneau bottom-left.
    pub bottom_left_mode: BottomLeftMode,

    /// Panneau avec focus.
    pub focus: FocusPanel,

    /// Index sélectionné (compatibilité - migrer vers graph_view.rows.selected_index()).
    pub selected_index: usize,

    /// État de la liste pour ratatui (compatibilité).
    pub graph_state: ListState,

    // ═══════════════════════════════════════════════════
    // Données associées au commit sélectionné (compatibilité)
    // ═══════════════════════════════════════════════════
    /// Fichiers du commit sélectionné.
    pub commit_files: Vec<DiffFile>,

    /// Index du fichier sélectionné (compatibilité - migrer vers graph_view.file_selected_index).
    pub file_selected_index: usize,

    /// Diff du fichier sélectionné.
    pub selected_file_diff: Option<crate::git::diff::FileDiff>,

    /// Offset de scroll dans le diff (compatibilité - migrer vers graph_view.diff_scroll_offset).
    pub diff_scroll_offset: usize,

    /// Mode d'affichage du diff (unifié ou côte à côte).
    pub diff_view_mode: DiffViewMode,

    /// Entrées de status (pour la vue staging, compatibilité).
    pub status_entries: Vec<StatusEntry>,

    /// Branches (compatibilité).
    pub branches: Vec<BranchInfo>,

    // ═══════════════════════════════════════════════════
    // Vues optionnelles (chargées à la demande)
    // ═══════════════════════════════════════════════════
    /// État de la vue staging.
    pub staging_state: StagingState,

    /// État de la vue branches.
    pub branches_view_state: BranchesViewState,

    /// État du blame (si actif).
    pub blame_state: Option<BlameState>,

    /// État de résolution de conflits (si actif).
    pub conflicts_state: Option<ConflictsState>,

    /// État de la recherche.
    pub search_state: SearchState,

    /// Picker de merge (si actif).
    pub merge_picker: Option<MergePickerState>,

    // ═══════════════════════════════════════════════════
    // UI transient
    // ═══════════════════════════════════════════════════
    /// Message flash à afficher.
    pub flash_message: Option<(String, Instant)>,

    /// Action en attente de confirmation (dialogue modal).
    pub pending_confirmation: Option<crate::ui::confirm_dialog::ConfirmAction>,

    /// Spinner de chargement actif.
    pub loading_spinner: Option<crate::ui::loading::LoadingSpinner>,

    /// Panneau de branches ouvert (compatibilité).
    pub show_branch_panel: bool,

    /// Index de la branche sélectionnée dans le panneau (compatibilité).
    pub branch_selected: usize,

    /// Flag pour quitter l'application.
    pub should_quit: bool,

    // ═══════════════════════════════════════════════════
    // Cache
    // ═══════════════════════════════════════════════════
    /// Cache des diffs.
    pub diff_cache: DiffCache,

    // ═══════════════════════════════════════════════════
    // Filtres pour le graph
    // ═══════════════════════════════════════════════════
    /// Filtres actifs sur le graph.
    pub graph_filter: GraphFilter,

    /// État du popup de filtre.
    pub filter_popup: FilterPopupState,
}

impl AppState {
    /// Crée un nouvel état d'application.
    pub fn new(repo: GitRepo, repo_path: String) -> crate::error::Result<Self> {
        let mut graph_state = ListState::default();
        graph_state.select(Some(0));

        let current_branch = repo.current_branch().ok();

        let state = Self {
            repo,
            repo_path,
            current_branch,
            view_mode: ViewMode::Graph,
            previous_view_mode: None,
            dirty: true,
            graph: Vec::new(),
            graph_view: GraphViewState::new(),
            bottom_left_mode: BottomLeftMode::Files,
            focus: FocusPanel::Graph,
            selected_index: 0,
            graph_state,
            commit_files: Vec::new(),
            file_selected_index: 0,
            selected_file_diff: None,
            diff_scroll_offset: 0,
            diff_view_mode: DiffViewMode::default(),
            status_entries: Vec::new(),
            branches: Vec::new(),
            staging_state: StagingState::new(),
            branches_view_state: BranchesViewState::new(),
            blame_state: None,
            conflicts_state: None,
            search_state: SearchState::default(),
            merge_picker: None,
            flash_message: None,
            pending_confirmation: None,
            loading_spinner: None,
            show_branch_panel: false,
            branch_selected: 0,
            should_quit: false,
            diff_cache: DiffCache::new(50),
            graph_filter: GraphFilter::new(),
            filter_popup: FilterPopupState::new(),
        };

        Ok(state)
    }

    /// Marque l'état comme nécessitant un refresh.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        // Vider le cache des diffs du working directory car ils peuvent être invalidés
        self.diff_cache.clear_working_directory();
    }

    /// L'état nécessite-t-il un refresh?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marque l'état comme propre.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Alias de mark_clean pour compatibilité.
    pub fn clear_dirty(&mut self) {
        self.mark_clean();
    }

    /// Définit un message flash.
    pub fn set_flash_message(&mut self, message: impl Into<String>) {
        self.flash_message = Some((message.into(), Instant::now()));
    }

    /// Efface le message flash.
    pub fn clear_flash_message(&mut self) {
        self.flash_message = None;
    }

    /// Vérifie si le message flash a expiré et le supprime le cas échéant.
    pub fn check_flash_expired(&mut self) {
        if let Some((_, timestamp)) = &self.flash_message {
            if timestamp.elapsed() > Duration::from_secs(3) {
                self.flash_message = None;
            }
        }
    }

    /// Retourne le message flash actuel s'il n'a pas expiré.
    pub fn current_flash_message(&self) -> Option<&str> {
        self.flash_message.as_ref().map(|(msg, _)| msg.as_str())
    }

    /// Retourne le commit actuellement sélectionné.
    pub fn selected_commit(&self) -> Option<&crate::git::graph::CommitNode> {
        self.graph.get(self.selected_index).map(|row| &row.node)
    }

    /// Accès au cache de diff.
    pub fn diff_cache_mut(&mut self) -> &mut DiffCache {
        &mut self.diff_cache
    }

    /// Met à jour la sélection du graph à partir de graph_view.
    pub fn sync_graph_selection(&mut self) {
        self.selected_index = self.graph_view.rows.selected_index();

        // Calcule l'index visuel correct en tenant compte du nombre d'items réels.
        // Chaque commit produit 1 item + 1 item de connexion (sauf le dernier qui n'a pas de connexion).
        // Donc pour N commits, on a 2*N - 1 items visuels.
        let visual_index = if self.graph.is_empty() {
            0
        } else {
            self.selected_index * 2
        };
        self.graph_state.select(Some(visual_index));
    }

    /// Met à jour graph_view à partir de la sélection legacy.
    /// Recharge également les fichiers du commit sélectionné.
    pub fn sync_legacy_selection(&mut self) {
        self.graph_view.rows.select(self.selected_index);
        // Recharger les fichiers du commit sélectionné
        if let Some(row) = self.graph.get(self.selected_index) {
            self.commit_files = self.repo.commit_diff(row.node.oid).unwrap_or_default();
            // Réinitialiser la sélection de fichier si nécessaire
            if self.file_selected_index >= self.commit_files.len() {
                self.file_selected_index = 0;
            }
            // Charger le diff du fichier sélectionné
            crate::handler::navigation::load_commit_file_diff(self);
        } else {
            self.commit_files.clear();
            self.file_selected_index = 0;
            self.selected_file_diff = None;
        }
    }
}

// Compatibilité: types exportés depuis l'ancien state.rs
pub use action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::state::selection::ListSelection;
    use git2::Oid;

    /// Helper pour créer un état de test avec un graph de taille donnée.
    fn create_test_state_with_graph(size: usize) -> AppState {
        // Créer un repo temporaire
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        // Configurer git
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        // Créer un commit initial
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();

        // Créer un graph de test
        let graph: Vec<GraphRow> = (0..size)
            .map(|i| GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[i as u8; 20]).unwrap_or(Oid::zero()),
                    message: format!("Commit {} message", i),
                    author: "Test Author".to_string(),
                    timestamp: i as i64 * 1000,
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![None],
                connection: if i + 1 < size {
                    // Tous les commits sauf le dernier ont une connexion
                    Some(crate::git::graph::ConnectionRow { cells: vec![] })
                } else {
                    None
                },
            })
            .collect();

        state.graph = graph;
        state.graph_view.rows = ListSelection::with_items(state.graph.clone());
        state.graph_view.rows.select(0);
        state.selected_index = 0;
        state.graph_state.select(Some(0));

        state
    }

    #[test]
    fn test_visual_index_last_commit() {
        // Créer un graph avec 2 commits
        let mut state = create_test_state_with_graph(2);

        // Sélectionner le dernier commit (index 1)
        state.selected_index = 1;
        state.graph_view.rows.select(1);

        // Appeler sync_graph_selection
        state.sync_graph_selection();

        // Vérifier que l'index visuel est correct : 1 * 2 = 2
        // Mais le nombre d'items visuels est 2*2 - 1 = 3 (indices 0, 1, 2)
        // Donc l'index visuel 2 est valide
        let visual_index = state.graph_state.selected().unwrap_or(999);
        assert_eq!(visual_index, 2);

        // Vérifier que l'index visuel est dans les bornes
        // Pour 2 commits : 2 items (commits) + 1 connexion = 3 items (indices 0 à 2)
        assert!(visual_index <= 2);
    }

    #[test]
    fn test_sync_graph_selection_empty() {
        // Créer un graph vide
        let mut state = create_test_state_with_graph(0);

        // Appeler sync_graph_selection avec un graphe vide
        state.sync_graph_selection();

        // Vérifier que l'index visuel est 0 (pas de panique)
        let visual_index = state.graph_state.selected().unwrap_or(999);
        assert_eq!(visual_index, 0);
    }

    #[test]
    fn test_sync_graph_selection_single_commit() {
        // Créer un graph avec 1 commit (pas de connexion)
        let mut state = create_test_state_with_graph(1);

        state.selected_index = 0;
        state.graph_view.rows.select(0);
        state.sync_graph_selection();

        // Pour 1 commit : 1 item seulement (pas de connexion)
        // L'index visuel devrait être 0
        let visual_index = state.graph_state.selected().unwrap_or(999);
        assert_eq!(visual_index, 0);
    }

    #[test]
    fn test_sync_graph_selection_multiple_commits() {
        // Créer un graph avec 5 commits
        let mut state = create_test_state_with_graph(5);

        // Tester différentes sélections
        for i in 0..5 {
            state.selected_index = i;
            state.graph_view.rows.select(i);
            state.sync_graph_selection();

            let visual_index = state.graph_state.selected().unwrap_or(999);
            // Pour chaque commit, l'index visuel est i * 2
            assert_eq!(visual_index, i * 2);

            // Vérifier que l'index visuel est dans les bornes
            // Pour 5 commits : 5 items (commits) + 4 connexions = 9 items (indices 0 à 8)
            assert!(visual_index <= 8);
        }
    }
}
