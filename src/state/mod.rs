//! État global de l'application.

pub mod action;
pub mod cache;
pub mod filter;
pub mod selection;
pub mod view;

pub use action::AppAction;
pub use cache::DiffCache;
pub use filter::{FilterField, FilterPopupState, GraphFilter};
pub use view::*;

use crate::git::branch::BranchInfo;
use crate::git::repo::{GitRepo, StatusEntry};
use ratatui::layout::Rect;
use std::time::{Duration, Instant};

/// Nombre initial de commits à charger (affichage rapide au démarrage).
pub const INITIAL_COMMIT_COUNT: usize = 200;
/// Nombre de commits supplémentaires à charger à chaque "charger plus".
pub const COMMIT_BATCH_SIZE: usize = 200;
/// Nombre maximum total de commits (limite de sécurité).
pub const MAX_TOTAL_COMMITS: usize = 10000;
/// Capacité du cache LRU pour les diffs (nombre d'entrées).
const DIFF_CACHE_CAPACITY: usize = 50;

/// État principal de l'application.
///
/// L'état du graphe et des sélections est centralisé dans `graph_view`.
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
    // Vue Graph (état unifié)
    // ═══════════════════════════════════════════════════
    /// État unifié de la vue graph (commits, sélections, diffs).
    pub graph_view: GraphViewState,

    /// Mode d'affichage du panneau bottom-left.
    pub bottom_left_mode: BottomLeftMode,

    /// Panneau avec focus.
    pub focus: FocusPanel,

    /// Dernière zone de rendu connue du terminal.
    pub screen_area: Rect,

    // ═══════════════════════════════════════════════════
    // Données complémentaires
    // ═══════════════════════════════════════════════════
    /// Entrées de status (pour la vue staging).
    pub status_entries: Vec<StatusEntry>,

    /// Branches (pour le panneau de branches).
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

    /// Picker de reset (si actif).
    pub reset_picker: Option<ResetPickerState>,

    // ═══════════════════════════════════════════════════
    // UI transient
    // ═══════════════════════════════════════════════════
    /// Message flash à afficher.
    pub flash_message: Option<(String, Instant)>,

    /// Action en attente de confirmation (dialogue modal).
    pub pending_confirmation: Option<crate::ui::confirm_dialog::ConfirmAction>,

    /// Spinner de chargement actif.
    pub loading_spinner: Option<crate::ui::loading::LoadingSpinner>,

    /// Panneau de branches ouvert.
    pub show_branch_panel: bool,

    /// Index de la branche sélectionnée dans le panneau.
    pub branch_selected: usize,

    /// Indique si un merge est en cours (MERGE_HEAD existe).
    pub is_merging: bool,

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
        let current_branch = repo.current_branch().ok();

        let state = Self {
            repo,
            repo_path,
            current_branch,
            view_mode: ViewMode::Graph,
            previous_view_mode: None,
            dirty: true,
            graph_view: GraphViewState::new(),
            bottom_left_mode: BottomLeftMode::Files,
            focus: FocusPanel::Graph,
            screen_area: Rect::default(),
            status_entries: Vec::new(),
            branches: Vec::new(),
            staging_state: StagingState::new(),
            branches_view_state: BranchesViewState::new(),
            blame_state: None,
            conflicts_state: None,
            search_state: SearchState::default(),
            merge_picker: None,
            reset_picker: None,
            flash_message: None,
            pending_confirmation: None,
            loading_spinner: None,
            show_branch_panel: false,
            branch_selected: 0,
            is_merging: false,
            should_quit: false,
            diff_cache: DiffCache::new(DIFF_CACHE_CAPACITY),
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

    /// Définit un message flash.
    pub fn set_flash_message(&mut self, message: impl Into<String>) {
        self.flash_message = Some((message.into(), Instant::now()));
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
        self.graph_view.selected_commit()
    }

    /// Remplace le graphe et met à jour l'état.
    ///
    /// Cette méthode garantit que tous les états restent synchronisés.
    pub fn replace_graph(&mut self, new_graph: Vec<crate::git::graph::GraphRow>) {
        self.graph_view.replace_graph(new_graph);
    }

    /// Rafraîchit les fichiers du commit sélectionné.
    ///
    /// Charge les fichiers depuis le repo et met à jour l'état.
    pub fn refresh_commit_files(&mut self) {
        if let Some(commit) = self.selected_commit() {
            let files = self.repo.commit_diff(commit.oid).unwrap_or_default();
            self.graph_view.set_commit_files(files);
        } else {
            self.graph_view.commit_files.clear();
            self.graph_view.file_selected_index = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::state::selection::ListSelection;
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
    fn test_selected_commit_uses_graph_view_selection() {
        let mut state = AppState::new(
            GitRepo::open(".").unwrap_or_else(|_| {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let mut opts = git2::RepositoryInitOptions::new();
                opts.initial_head("main");
                let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();
                let mut config = repo.config().unwrap();
                config.set_str("user.name", "Test").unwrap();
                config.set_str("user.email", "test@test.com").unwrap();
                let sig = git2::Signature::now("Test", "test@test.com").unwrap();
                let mut index = repo.index().unwrap();
                let tree_oid = index.write_tree().unwrap();
                let tree = repo.find_tree(tree_oid).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                    .unwrap();
                GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap()
            }),
            ".".to_string(),
        )
        .unwrap();

        state.graph_view.rows = ListSelection::with_items(create_test_graph(5));
        state.graph_view.rows.select(2);

        assert_eq!(
            state
                .selected_commit()
                .map(|commit| commit.message.as_str()),
            Some("Commit 2")
        );
    }

    #[test]
    fn test_replace_graph_updates_graph_view() {
        let mut state = AppState::new(
            GitRepo::open(".").unwrap_or_else(|_| {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let mut opts = git2::RepositoryInitOptions::new();
                opts.initial_head("main");
                let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();
                let mut config = repo.config().unwrap();
                config.set_str("user.name", "Test").unwrap();
                config.set_str("user.email", "test@test.com").unwrap();
                let sig = git2::Signature::now("Test", "test@test.com").unwrap();
                let mut index = repo.index().unwrap();
                let tree_oid = index.write_tree().unwrap();
                let tree = repo.find_tree(tree_oid).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                    .unwrap();
                GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap()
            }),
            ".".to_string(),
        )
        .unwrap();

        state.replace_graph(create_test_graph(10));

        assert_eq!(state.graph_view.len(), 10);
    }

    #[test]
    fn test_graph_view_visual_index_empty_graph() {
        let state = AppState::new(
            GitRepo::open(".").unwrap_or_else(|_| {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let mut opts = git2::RepositoryInitOptions::new();
                opts.initial_head("main");
                let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();
                let mut config = repo.config().unwrap();
                config.set_str("user.name", "Test").unwrap();
                config.set_str("user.email", "test@test.com").unwrap();
                let sig = git2::Signature::now("Test", "test@test.com").unwrap();
                let mut index = repo.index().unwrap();
                let tree_oid = index.write_tree().unwrap();
                let tree = repo.find_tree(tree_oid).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                    .unwrap();
                GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap()
            }),
            ".".to_string(),
        )
        .unwrap();

        assert_eq!(state.graph_view.visual_index(), 0);
    }
}
