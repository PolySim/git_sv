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

use crate::git::repo::{GitRepo, StatusEntry};
use ratatui::layout::Rect;
use std::time::{Duration, Instant};

/// Nombre initial de commits à chargér (affichage rapide au démarrage).
pub const INITIAL_COMMIT_COUNT: usize = 200;
/// Nombre de commits supplémentaires à chargér à chaque "chargér plus".
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

    /// Spinner de chargément actif.
    pub loading_spinner: Option<crate::ui::loading::LoadingSpinner>,

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

    /// Planifie un rafraîchissement sans invalider explicitement le cache diff.
    pub fn schedule_refresh(&mut self) {
        self.dirty = true;
    }

    /// Met à jour la zone d'écran connue après rendu.
    pub fn update_screen_area(&mut self, area: Rect) {
        self.screen_area = area;
    }

    /// Bascule vers une vue et planifie un rafraîchissement.
    pub fn enter_view(&mut self, view_mode: ViewMode) {
        self.view_mode = view_mode;
        self.schedule_refresh();
    }

    /// Bascule l'overlay d'aide en préservant la vue précédente.
    pub fn toggle_help(&mut self) {
        if self.view_mode == ViewMode::Help {
            self.leave_help();
        } else {
            self.previous_view_mode = Some(self.view_mode);
            self.view_mode = ViewMode::Help;
        }
    }

    /// Quitte la vue d'aide et restaure la vue précédente.
    pub fn leave_help(&mut self) {
        self.view_mode = self.previous_view_mode.take().unwrap_or(ViewMode::Graph);
    }

    /// Ouvre une confirmation modale.
    pub fn open_confirmation(&mut self, action: crate::ui::confirm_dialog::ConfirmAction) {
        self.pending_confirmation = Some(action);
    }

    /// Ferme la confirmation modale courante.
    pub fn close_confirmation(&mut self) {
        self.pending_confirmation = None;
    }

    /// Active le spinner de chargement.
    pub fn set_loading(&mut self, message: impl Into<String>) {
        self.loading_spinner = Some(crate::ui::loading::LoadingSpinner::new(message.into()));
    }

    /// Désactive le spinner de chargement.
    pub fn clear_loading(&mut self) {
        self.loading_spinner = None;
    }

    /// Demande la fermeture de l'application.
    pub fn request_quit(&mut self) {
        self.should_quit = true;
    }

    /// Ouvre la vue blame avec son état.
    pub fn open_blame(&mut self, blame_state: BlameState) {
        self.blame_state = Some(blame_state);
        self.view_mode = ViewMode::Blame;
    }

    /// Ferme la vue blame et revient au graphe.
    pub fn close_blame(&mut self) {
        self.blame_state = None;
        self.view_mode = ViewMode::Graph;
    }

    /// Ouvre la vue conflits avec son état.
    pub fn open_conflicts(&mut self, conflicts_state: ConflictsState) {
        self.conflicts_state = Some(conflicts_state);
        self.view_mode = ViewMode::Conflicts;
    }

    /// Ferme la vue conflits sans changer d'autre état.
    pub fn clear_conflicts(&mut self) {
        self.conflicts_state = None;
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

    /// Initialise l'état à partir du repository.
    ///
    /// Invariants en sortie:
    /// - la sélection du graphe reste valide ;
    /// - la pagination reflète les commits chargés ;
    /// - `status_entries` et `staging_state` sont synchronisés ;
    /// - le diff du commit ou du staging est cohérent avec la vue active ;
    pub fn initialize_from_repo(&mut self) -> crate::error::Result<()> {
        self.refresh_with_commit_limit(INITIAL_COMMIT_COUNT)
    }

    /// Rafraîchit l'état courant à partir du repository, en préservant
    /// le niveau de pagination déjà chargé quand c'est possible.
    pub fn refresh_from_repo(&mut self) -> crate::error::Result<()> {
        let commit_limit = self.graph_view.loaded_count.max(INITIAL_COMMIT_COUNT);
        self.refresh_with_commit_limit(commit_limit)
    }

    fn refresh_with_commit_limit(&mut self, commit_limit: usize) -> crate::error::Result<()> {
        self.current_branch = self.repo.current_branch().ok();

        let (new_graph, can_load_more) = if self.graph_filter.is_active() {
            self.repo
                .build_graph_filtered_with_more(commit_limit, &self.graph_filter)
                .unwrap_or_default()
        } else {
            self.repo
                .build_graph_with_more(commit_limit)
                .unwrap_or_default()
        };

        let graph_len = new_graph.len();
        self.replace_graph(new_graph);

        let total = if self.graph_filter.is_active() {
            None
        } else {
            self.repo.estimate_total_commits()
        };
        self.graph_view
            .update_pagination_state(graph_len, total, can_load_more);

        self.refresh_commit_files();
        self.refresh_selected_commit_diff();

        let status_entries = self.repo.status().unwrap_or_default();
        self.apply_status_entries(status_entries);

        if self.view_mode == ViewMode::Branches {
            self.refresh_branches_view_data();
        }

        if self.view_mode == ViewMode::Staging {
            crate::handler::staging::load_staging_diff(self);
        }

        self.is_merging = crate::git::conflict::is_merging(&self.repo.repo);
        self.dirty = false;

        Ok(())
    }

    /// Synchronise `status_entries` et la vue staging à partir d'une seule lecture.
    pub fn apply_status_entries(&mut self, status_entries: Vec<StatusEntry>) {
        let staged_files = status_entries
            .iter()
            .filter(|entry| entry.is_staged())
            .cloned()
            .collect();
        let unstaged_files = status_entries
            .iter()
            .filter(|entry| entry.is_unstaged())
            .cloned()
            .collect();

        self.status_entries = status_entries;
        self.staging_state.set_staged_files(staged_files);
        self.staging_state.set_unstaged_files(unstaged_files);

        if self.staging_state.unstaged_selected() >= self.staging_state.unstaged_files().len() {
            let new_idx = self.staging_state.unstaged_files().len().saturating_sub(1);
            self.staging_state.set_unstaged_selected(new_idx);
        }

        if self.staging_state.staged_selected() >= self.staging_state.staged_files().len() {
            let new_idx = self.staging_state.staged_files().len().saturating_sub(1);
            self.staging_state.set_staged_selected(new_idx);
        }
    }

    fn refresh_selected_commit_diff(&mut self) {
        if !self.graph_view.commit_files.is_empty() {
            crate::handler::navigation::load_commit_file_diff(self);
        } else {
            self.graph_view.clear_file_diff();
        }
    }

    fn refresh_branches_view_data(&mut self) {
        match crate::git::branch::list_all_branches(&self.repo.repo) {
            Ok((local, remote)) => {
                self.branches_view_state.local_branches.set_items(local);
                self.branches_view_state.remote_branches.set_items(remote);
            }
            Err(e) => {
                self.set_flash_message(format!("Erreur chargement branches: {}", e));
            }
        }

        if let Ok(worktrees) = crate::git::worktree::list_worktrees(&self.repo.repo) {
            self.branches_view_state.worktrees.set_items(worktrees);
        }

        if let Ok(stashes) = crate::git::stash::list_stashes(&mut self.repo.repo) {
            self.branches_view_state.stashes.set_items(stashes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::commit_file;
    use crate::state::selection::ListSelection;
    use git2::Oid;
    use std::path::Path;

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

    fn create_test_state() -> (tempfile::TempDir, AppState) {
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

        let git_repo = GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap();
        let state = AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
        (temp_dir, state)
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

    #[test]
    fn test_toggle_help_roundtrip() {
        let (_temp_dir, mut state) = create_test_state();
        state.view_mode = ViewMode::Branches;

        state.toggle_help();
        assert_eq!(state.view_mode, ViewMode::Help);
        assert_eq!(state.previous_view_mode, Some(ViewMode::Branches));

        state.toggle_help();
        assert_eq!(state.view_mode, ViewMode::Branches);
        assert_eq!(state.previous_view_mode, None);
    }

    #[test]
    fn test_confirmation_helpers() {
        let (_temp_dir, mut state) = create_test_state();

        state.open_confirmation(crate::ui::confirm_dialog::ConfirmAction::DiscardAll);
        assert!(state.pending_confirmation.is_some());

        state.close_confirmation();
        assert!(state.pending_confirmation.is_none());
    }

    #[test]
    fn test_loading_helpers() {
        let (_temp_dir, mut state) = create_test_state();

        state.set_loading("Chargement");
        assert!(state.loading_spinner.is_some());

        state.clear_loading();
        assert!(state.loading_spinner.is_none());
    }

    #[test]
    fn test_refresh_from_repo_preserves_selection() {
        let (temp_dir, mut state) = create_test_state();
        commit_file(&state.repo.repo, "a.txt", "one\n", "Commit A");
        commit_file(&state.repo.repo, "b.txt", "two\n", "Commit B");

        state.initialize_from_repo().unwrap();
        state.graph_view.select_commit(1);

        let selected_oid = state.selected_commit().map(|commit| commit.oid).unwrap();

        std::fs::write(temp_dir.path().join("scratch.txt"), "scratch\n").unwrap();
        state.refresh_from_repo().unwrap();

        assert_eq!(
            state.selected_commit().map(|commit| commit.oid),
            Some(selected_oid)
        );
    }

    #[test]
    fn test_refresh_from_repo_reloads_selected_commit_diff() {
        let (_temp_dir, mut state) = create_test_state();
        commit_file(
            &state.repo.repo,
            "tracked.txt",
            "content\n",
            "Commit with file",
        );

        state.initialize_from_repo().unwrap();
        assert!(!state.graph_view.commit_files.is_empty());
        assert!(state.graph_view.selected_file_diff.is_some());

        state.graph_view.selected_file_diff = None;
        state.refresh_from_repo().unwrap();

        assert!(state.graph_view.selected_file_diff.is_some());
    }

    #[test]
    fn test_apply_status_entries_keeps_status_and_staging_in_sync() {
        let (temp_dir, mut state) = create_test_state();
        commit_file(&state.repo.repo, "tracked.txt", "base\n", "Add tracked");

        std::fs::write(temp_dir.path().join("tracked.txt"), "base\nmod\n").unwrap();
        std::fs::write(temp_dir.path().join("staged.txt"), "staged\n").unwrap();

        let mut index = state.repo.repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();

        state.initialize_from_repo().unwrap();

        let total_staging_entries =
            state.staging_state.staged_files().len() + state.staging_state.unstaged_files().len();

        assert_eq!(state.status_entries.len(), total_staging_entries);
        assert!(state
            .staging_state
            .unstaged_files()
            .iter()
            .any(|entry| entry.path == "tracked.txt"));
        assert!(state
            .staging_state
            .staged_files()
            .iter()
            .any(|entry| entry.path == "staged.txt"));
    }

    #[test]
    fn test_refresh_from_repo_filtered_graph_uses_filtered_pagination() {
        let (_temp_dir, mut state) = create_test_state();
        commit_file(&state.repo.repo, "alpha.txt", "one\n", "Alpha commit");
        commit_file(&state.repo.repo, "beta.txt", "two\n", "Beta commit");

        state.graph_filter.message = Some("Alpha".to_string());
        state.initialize_from_repo().unwrap();

        assert_eq!(state.graph_view.len(), 1);
        assert_eq!(state.graph_view.total_commits, None);
        assert_eq!(
            state
                .selected_commit()
                .map(|commit| commit.message.as_str()),
            Some("Alpha commit")
        );
    }
}
