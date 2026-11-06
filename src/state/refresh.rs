//! Invalidation et chargement à la demande des données propres à chaque vue.

use super::{AppState, ViewMode, INITIAL_COMMIT_COUNT};

pub(super) struct PendingRefresh {
    pub metadata: bool,
    pub graph: bool,
    pub status: bool,
    pub branches: bool,
    pub project_tree: bool,
}

impl PendingRefresh {
    pub fn all() -> Self {
        Self {
            metadata: true,
            graph: true,
            status: true,
            branches: true,
            project_tree: true,
        }
    }
}

impl AppState {
    /// Rafraîchit explicitement le dépôt, en préservant la pagination chargée.
    #[cfg(test)]
    pub fn refresh_from_repo(&mut self) -> crate::error::Result<()> {
        self.mark_dirty();
        let commit_limit = self.graph_view.loaded_count.max(INITIAL_COMMIT_COUNT);
        self.refresh_with_commit_limit(commit_limit, true)
    }

    /// Recharge uniquement les données invalidées nécessaires à la vue visible.
    pub fn refresh_pending(&mut self) -> crate::error::Result<()> {
        let commit_limit = self.graph_view.loaded_count.max(INITIAL_COMMIT_COUNT);
        self.refresh_with_commit_limit(commit_limit, false)
    }

    /// Consomme une demande de scan explicite sans la répéter à chaque changement d'onglet.
    pub fn take_status_refresh_request(&mut self) -> bool {
        self.background_status && std::mem::take(&mut self.status_refresh_requested)
    }

    /// Indique que le statut affiché attend la prochaine collecte du watcher.
    pub fn status_loading(&self) -> bool {
        self.background_status && self.pending_refresh.status
    }

    /// Applique une collecte complète du worker et invalide les vues du working tree.
    pub fn accept_background_status(&mut self, entries: Vec<crate::git::repo::StatusEntry>) {
        self.apply_status_entries(entries);
        self.status_refresh_requested = false;
        self.diff_cache.clear_working_directory();
        self.dirty = true;
    }

    /// Conserve une erreur de lecture pendant que le watcher poursuit ses tentatives.
    pub fn report_status_error(&mut self, error: String) {
        self.status_error = Some(crate::utils::flash_error("status", error));
        self.dirty = true;
    }

    pub(super) fn refresh_with_commit_limit(
        &mut self,
        commit_limit: usize,
        synchronous: bool,
    ) -> crate::error::Result<()> {
        if self.pending_refresh.metadata {
            self.refresh_repository_metadata();
        }
        if self.pending_refresh.graph && (synchronous || self.view_mode == ViewMode::Graph) {
            self.refresh_graph(commit_limit);
        }
        if self.pending_refresh.status && (synchronous || !self.background_status) {
            match self.repo.status() {
                Ok(entries) => self.apply_status_entries(entries),
                Err(error) => self.set_flash_message(crate::utils::flash_error("status", error)),
            }
        }

        match self.view_mode {
            ViewMode::Branches if self.pending_refresh.branches => {
                self.refresh_branches_view_data();
                self.pending_refresh.branches = false;
            }
            ViewMode::Staging if !self.pending_refresh.status => {
                crate::handler::staging::load_staging_diff(self);
            }
            ViewMode::ProjectTree if !self.pending_refresh.status => {
                if self.pending_refresh.project_tree {
                    self.refresh_project_tree_from_status();
                    self.pending_refresh.project_tree = false;
                }
                self.ensure_project_tree_focus_loaded();
            }
            _ => {}
        }
        self.dirty = false;
        Ok(())
    }

    fn refresh_repository_metadata(&mut self) {
        let previous_branch = self.current_branch.clone();
        self.current_branch = match self.repo.current_branch() {
            Ok(branch) => Some(branch),
            Err(error) => {
                self.set_flash_message(crate::utils::flash_error("branche courante", error));
                None
            }
        };
        if previous_branch.is_some() && previous_branch != self.current_branch {
            self.project_tree_state.clear_comparison();
        }
        self.ui.is_merging = crate::git::conflict::is_merging(&self.repo.repo);
        self.ui.is_bisecting = crate::git::bisect::is_active(&self.repo.repo);
        self.pending_refresh.metadata = false;
    }

    fn refresh_graph(&mut self, commit_limit: usize) {
        let filtered = self.filters.graph_filter.is_active();
        let result = if filtered {
            self.repo
                .build_graph_filtered_with_more(commit_limit, &self.filters.graph_filter)
        } else {
            self.repo.build_graph_with_more(commit_limit)
        };
        let (graph, can_load_more) = match result {
            Ok(result) => result,
            Err(error) => {
                self.set_flash_message(crate::utils::flash_error("chargement graphe", error));
                return;
            }
        };
        let graph_len = graph.len();
        self.replace_graph(graph);
        let total = if filtered {
            None
        } else {
            self.repo.estimate_total_commits()
        };
        self.graph_view
            .update_pagination_state(graph_len, total, can_load_more);
        self.refresh_commit_files();
        self.refresh_selected_commit_diff();
        self.pending_refresh.graph = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    fn state_with_commit() -> (tempfile::TempDir, AppState) {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "tracked.txt", "initial\n", "initial");
        let state = AppState::new(GitRepo { repo }, temp.path().display().to_string()).unwrap();
        (temp, state)
    }

    #[test]
    fn startup_defers_status_without_claiming_the_worktree_is_clean() {
        let (temp, mut state) = state_with_commit();
        std::fs::write(temp.path().join("new.txt"), "untracked").unwrap();
        state.initialize_for_tui().unwrap();
        assert!(!state.graph_view.is_empty());
        assert!(state.status_loading());
        assert!(!state.status_loaded);
        assert!(!state.take_status_refresh_request());

        state.enter_view(ViewMode::Staging);
        state.refresh_pending().unwrap();
        assert!(state.status_loading());
        assert!(!state.take_status_refresh_request());
        let entries = state.repo.status().unwrap();
        state.accept_background_status(entries);
        state.refresh_pending().unwrap();
        assert!(!state.status_loading());
        assert!(state.status_loaded);
        assert_eq!(state.staging_state.unstaged_files()[0].path, "new.txt");
    }

    #[test]
    fn switching_tabs_reuses_status_until_repository_invalidation() {
        let (temp, mut state) = state_with_commit();
        state.initialize_from_repo().unwrap();
        std::fs::write(temp.path().join("new.txt"), "untracked").unwrap();
        for view in [ViewMode::Staging, ViewMode::Branches, ViewMode::Graph] {
            state.enter_view(view);
            state.refresh_pending().unwrap();
            assert!(state.status_entries.is_empty());
        }
        state.mark_dirty();
        state.refresh_pending().unwrap();
        assert_eq!(state.status_entries[0].path, "new.txt");
    }

    #[test]
    fn filter_refresh_does_not_rescan_worktree() {
        let (temp, mut state) = state_with_commit();
        state.initialize_from_repo().unwrap();
        std::fs::write(temp.path().join("new.txt"), "untracked").unwrap();
        state.schedule_refresh();
        state.refresh_pending().unwrap();
        assert!(state.status_entries.is_empty());
        state.mark_dirty();
        state.refresh_pending().unwrap();
        assert_eq!(state.status_entries[0].path, "new.txt");
    }

    #[test]
    fn invalidated_inactive_views_reload_when_entered() {
        let (temp, mut state) = state_with_commit();
        state.initialize_from_repo().unwrap();
        state.enter_view(ViewMode::Branches);
        state.refresh_pending().unwrap();
        state.enter_view(ViewMode::ProjectTree);
        state.refresh_pending().unwrap();
        state.enter_view(ViewMode::Graph);
        let head = state.repo.repo.head().unwrap().peel_to_commit().unwrap();
        state.repo.repo.branch("new-branch", &head, false).unwrap();
        drop(head);
        std::fs::remove_file(temp.path().join("tracked.txt")).unwrap();
        std::fs::write(temp.path().join("new.txt"), "untracked").unwrap();
        state.mark_dirty();
        state.refresh_pending().unwrap();
        assert!(!state
            .branches_view_state
            .local_branches
            .iter()
            .any(|b| b.name == "new-branch"));
        state.enter_view(ViewMode::Branches);
        state.refresh_pending().unwrap();
        assert!(state
            .branches_view_state
            .local_branches
            .iter()
            .any(|b| b.name == "new-branch"));
        state.enter_view(ViewMode::ProjectTree);
        state.refresh_pending().unwrap();
        let entries = state.project_tree_state.entries.items();
        assert!(entries.iter().any(|entry| entry.path == "new.txt"));
        assert!(!entries.iter().any(|entry| entry.path == "tracked.txt"));
    }

    #[test]
    fn closing_help_or_blame_applies_pending_repository_changes() {
        let (_temp, mut state) = state_with_commit();
        state.initialize_from_repo().unwrap();
        for view in [ViewMode::Help, ViewMode::Blame] {
            let old_count = state.graph_view.len();
            state.view_mode = view;
            commit_file(&state.repo.repo, "tracked.txt", &format!("{view:?}"), "new");
            state.mark_dirty();
            state.refresh_pending().unwrap();
            assert_eq!(state.graph_view.len(), old_count);
            match view {
                ViewMode::Help => state.leave_help(),
                _ => state.close_blame(),
            }
            assert!(state.dirty);
            state.refresh_pending().unwrap();
            assert_eq!(state.graph_view.len(), old_count + 1);
        }
    }

    #[test]
    fn synchronous_status_after_mutation_keeps_the_watcher_reset_request() {
        let (_temp, mut state) = state_with_commit();
        state.initialize_for_tui().unwrap();
        state.mark_dirty();
        state.apply_status_entries(Vec::new());
        assert!(state.take_status_refresh_request());
        assert!(!state.take_status_refresh_request());
    }

    #[test]
    fn status_errors_remain_visible_until_a_successful_collection() {
        let (_temp, mut state) = state_with_commit();
        state.initialize_for_tui().unwrap();
        state.report_status_error("cannot read index".into());
        assert!(state
            .current_flash_message()
            .unwrap()
            .contains("cannot read index"));
        state.set_flash_message("temporary");
        state.ui.flash_message = None;
        assert!(state
            .current_flash_message()
            .unwrap()
            .contains("cannot read index"));
        state.accept_background_status(Vec::new());
        assert!(state.current_flash_message().is_none());
        assert!(!state.status_loading());
    }

    #[cfg(feature = "profiling")]
    #[test]
    #[ignore = "Profilage en lecture seule ; définir GIT_SV_PROFILE_REPO"]
    fn profile_large_repository_lifecycle() {
        use std::time::Instant;

        let path = std::env::var("GIT_SV_PROFILE_REPO").expect("GIT_SV_PROFILE_REPO requis");
        let start = Instant::now();
        let mut state = AppState::new(GitRepo::open(&path).unwrap(), path).unwrap();
        state.initialize_for_tui().unwrap();
        eprintln!("initial_graph_us={}", start.elapsed().as_micros());
        // La mesure de cette collecte est séparée : elle s'exécute dans le watcher en TUI.
        let start = Instant::now();
        let entries = state.repo.status().unwrap();
        eprintln!("status_collection_us={}", start.elapsed().as_micros());
        state.accept_background_status(entries);
        state.refresh_pending().unwrap();
        let backend = ratatui::backend::TestBackend::new(160, 48);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        for pass in 0..3 {
            for view in [
                ViewMode::Staging,
                ViewMode::Branches,
                ViewMode::ProjectTree,
                ViewMode::Graph,
            ] {
                let start = Instant::now();
                state.enter_view(view);
                state.refresh_pending().unwrap();
                terminal
                    .draw(|frame| crate::ui::render(frame, &mut state))
                    .unwrap();
                eprintln!(
                    "tab_{view:?}_pass_{pass}_us={}",
                    start.elapsed().as_micros()
                );
            }
        }
        let mut dispatcher = crate::handler::ActionDispatcher::new();
        let start = Instant::now();
        for _ in 0..20 {
            dispatcher
                .dispatch(
                    &mut state,
                    crate::state::AppAction::Navigation(
                        crate::state::action::NavigationAction::MoveDown,
                    ),
                )
                .unwrap();
            terminal
                .draw(|frame| crate::ui::render(frame, &mut state))
                .unwrap();
        }
        eprintln!("graph_20_moves_us={}", start.elapsed().as_micros());
    }
}
