//! Chargement paresseux des panneaux de la vue arborescence.

use super::project_history_loader::{
    ProjectHistoryData, ProjectHistoryLoader, ProjectHistoryRequest,
};
use super::{AppState, ProjectTreeFocus, ViewMode, MAX_TOTAL_COMMITS};

impl AppState {
    /// Réutilise le statut complet déjà collecté au lieu de rescanner le working tree.
    pub(super) fn refresh_project_tree_from_status(&mut self) {
        match crate::git::project_tree::current_project_files_from_status(
            &self.repo.repo,
            &self.status_entries,
        ) {
            Ok(files) => self.project_tree_state.set_files(files),
            Err(error) => {
                self.set_flash_message(crate::utils::flash_error("arborescence", error));
            }
        }
    }

    /// Rafraîchit l'arborescence sans charger les panneaux dépendants.
    #[cfg(test)]
    pub fn refresh_project_tree(&mut self) {
        match self.repo.current_project_files() {
            Ok(files) => self.project_tree_state.set_files(files),
            Err(error) => {
                self.set_flash_message(crate::utils::flash_error("arborescence", error));
            }
        }
    }

    /// Recharge l'historique du fichier ou dossier sélectionné.
    pub fn refresh_selected_path_history(&mut self) {
        let Some(entry) = self.project_tree_state.selected_entry().cloned() else {
            self.project_tree_state.invalidate_path_history();
            if let Some(loader) = &self.project_history_loader {
                loader.cancel();
            }
            return;
        };
        if self.background_status {
            if self.project_tree_state.history_loaded {
                self.project_tree_state.invalidate_path_history();
            }
            if let Err(error) = self.request_project_history() {
                self.project_tree_state.invalidate_path_history();
                self.set_flash_message(crate::utils::flash_error("historique du chemin", error));
            }
            return;
        }

        let comparison_target = self
            .project_tree_state
            .comparison
            .as_ref()
            .map(|comparison| comparison.target_branch.clone());
        if let Some(target_branch) = comparison_target {
            match self.repo.compare_path_history(
                &entry.path,
                entry.is_directory(),
                &target_branch,
                MAX_TOTAL_COMMITS,
            ) {
                Ok(comparison) => self
                    .project_tree_state
                    .set_compared_path_history(comparison),
                Err(error) => {
                    self.project_tree_state.invalidate_path_history();
                    self.set_flash_message(crate::utils::flash_error(
                        "comparaison de l'historique du chemin",
                        error,
                    ));
                }
            }
            return;
        }

        match self
            .repo
            .path_history(&entry.path, entry.is_directory(), MAX_TOTAL_COMMITS)
        {
            Ok(history) => self.project_tree_state.set_path_history(history),
            Err(error) => {
                self.project_tree_state.invalidate_path_history();
                self.set_flash_message(crate::utils::flash_error("historique du chemin", error));
            }
        }
    }

    fn current_project_history_request(
        &self,
    ) -> crate::error::Result<Option<ProjectHistoryRequest>> {
        let Some(entry) = self.project_tree_state.selected_entry() else {
            return Ok(None);
        };
        let head = match self.repo.repo.head() {
            Ok(head) => Some(head.peel_to_commit()?.id()),
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
            Err(error) => return Err(error.into()),
        };
        let comparison = self
            .project_tree_state
            .comparison
            .as_ref()
            .map(|comparison| {
                self.repo
                    .repo
                    .revparse_single(&comparison.target_branch)?
                    .peel_to_commit()
                    .map(|target| (comparison.target_branch.clone(), target.id()))
            })
            .transpose()?;
        Ok(Some(ProjectHistoryRequest {
            repo_path: self.repo_path.clone(),
            path: entry.path.clone(),
            is_directory: entry.is_directory(),
            generation: self.project_tree_state.history_generation,
            head,
            comparison,
        }))
    }

    fn request_project_history(&mut self) -> crate::error::Result<()> {
        let Some(request) = self.current_project_history_request()? else {
            return Ok(());
        };
        if self.project_history_loader.is_none() {
            self.project_history_loader = Some(ProjectHistoryLoader::new()?);
        }
        if let Some(loader) = &self.project_history_loader {
            loader.request(request);
            self.project_tree_state.history_loading = true;
        }
        Ok(())
    }

    /// Indique si la boucle TUI doit récupérer un résultat du worker prochainement.
    pub fn project_history_pending(&self) -> bool {
        self.project_history_loader
            .as_ref()
            .is_some_and(ProjectHistoryLoader::pending)
    }

    /// Applique uniquement un résultat correspondant encore au chemin et aux références actifs.
    pub fn poll_project_history(&mut self) -> bool {
        let Some(loader) = &self.project_history_loader else {
            return false;
        };
        if !self.project_tree_state.history_loading {
            loader.cancel();
            return false;
        }
        let Some(result) = loader.take_completed() else {
            return false;
        };
        if self
            .current_project_history_request()
            .ok()
            .flatten()
            .as_ref()
            != Some(&result.request)
        {
            self.project_tree_state.history_loading = false;
            if self.view_mode == ViewMode::ProjectTree {
                self.ensure_project_tree_focus_loaded();
            }
            return true;
        }
        match result.data {
            Ok(ProjectHistoryData::History(history)) => {
                self.project_tree_state.set_path_history(history);
            }
            Ok(ProjectHistoryData::Comparison(comparison)) => {
                self.project_tree_state
                    .set_compared_path_history(comparison);
            }
            Err(error) => {
                self.project_tree_state.invalidate_path_history();
                self.set_flash_message(crate::utils::flash_error("historique du chemin", error));
                return true;
            }
        }
        if self.view_mode == ViewMode::ProjectTree {
            self.ensure_project_tree_focus_loaded();
        }
        true
    }

    /// Recharge les fichiers touchés par le commit d'historique sélectionné.
    pub fn refresh_selected_history_commit_details(&mut self) {
        if !self.project_tree_state.history_loaded {
            return;
        }
        let Some(oid) = self
            .project_tree_state
            .selected_history_commit()
            .map(|commit| commit.oid)
        else {
            self.project_tree_state.set_changed_files(Vec::new());
            return;
        };

        match self.repo.commit_diff(oid) {
            Ok(files) => self.project_tree_state.set_changed_files(files),
            Err(error) => {
                self.project_tree_state.invalidate_commit_details();
                self.set_flash_message(crate::utils::flash_error("fichiers du commit", error));
            }
        }
    }

    /// Recharge le diff du fichier sélectionné dans le commit d'historique.
    pub fn refresh_selected_history_file_diff(&mut self) {
        if !self.project_tree_state.history_loaded || !self.project_tree_state.commit_details_loaded
        {
            return;
        }
        let commit_oid = self
            .project_tree_state
            .selected_history_commit()
            .map(|commit| commit.oid);
        let file_path = self
            .project_tree_state
            .selected_changed_file()
            .map(|file| file.path.clone());
        let (Some(commit_oid), Some(file_path)) = (commit_oid, file_path) else {
            self.project_tree_state.set_selected_diff(None);
            return;
        };

        let cache_key = crate::state::cache::DiffCacheKey::new(commit_oid, &file_path);
        if let Some(diff) = self.diff_cache.get(&cache_key) {
            self.project_tree_state.set_selected_diff(Some(diff));
            return;
        }

        match self.repo.file_diff(commit_oid, &file_path) {
            Ok(diff) => {
                let diff = std::sync::Arc::new(diff);
                self.diff_cache.put(cache_key, diff.clone());
                self.project_tree_state.set_selected_diff(Some(diff));
            }
            Err(error) => {
                self.project_tree_state.invalidate_diff();
                self.set_flash_message(crate::utils::flash_error("diff du commit", error));
            }
        }
    }

    /// Charge uniquement les données nécessaires au panneau actif.
    pub fn ensure_project_tree_focus_loaded(&mut self) {
        match self.project_tree_state.focus {
            ProjectTreeFocus::Tree => {}
            ProjectTreeFocus::History => {
                if !self.project_tree_state.history_loaded {
                    self.refresh_selected_path_history();
                }
            }
            ProjectTreeFocus::ChangedFiles => {
                if !self.project_tree_state.history_loaded {
                    self.refresh_selected_path_history();
                }
                if !self.project_tree_state.history_loaded {
                    return;
                }
                if !self.project_tree_state.commit_details_loaded {
                    self.refresh_selected_history_commit_details();
                }
            }
            ProjectTreeFocus::Diff => {
                if !self.project_tree_state.history_loaded {
                    self.refresh_selected_path_history();
                }
                if !self.project_tree_state.history_loaded {
                    return;
                }
                if !self.project_tree_state.commit_details_loaded {
                    self.refresh_selected_history_commit_details();
                }
                if !self.project_tree_state.diff_loaded {
                    self.refresh_selected_history_file_diff();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::time::{Duration, Instant};

    fn async_state() -> (tempfile::TempDir, AppState) {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "initial", "initial");
        let mut state = AppState::new(GitRepo { repo }, temp.path().display().to_string()).unwrap();
        state.refresh_project_tree();
        state.background_status = true;
        state.view_mode = ViewMode::ProjectTree;
        state.project_tree_state.focus = ProjectTreeFocus::History;
        (temp, state)
    }

    #[test]
    #[ignore = "mesure manuelle en lecture seule via GIT_SV_PROFILE_REPO"]
    fn profile_async_project_history() {
        let path = std::env::var("GIT_SV_PROFILE_REPO").expect("GIT_SV_PROFILE_REPO requis");
        let repo = GitRepo::open(&path).unwrap();
        let index = repo.repo.index().unwrap();
        let entry = index.get(index.len() / 2).unwrap();
        let file = String::from_utf8_lossy(&entry.path).into_owned();
        let mut state = AppState::new(repo, path).unwrap();
        state.background_status = true;
        state.view_mode = ViewMode::ProjectTree;
        state.project_tree_state.focus = ProjectTreeFocus::History;
        state.project_tree_state.set_files(vec![file]);
        while state
            .project_tree_state
            .selected_entry()
            .is_some_and(|entry| entry.is_directory())
        {
            let selected = state.project_tree_state.entries.selected_index();
            state.project_tree_state.expand_selected_directory();
            state.project_tree_state.select_tree_entry(selected + 1);
        }
        let started = Instant::now();
        state.refresh_selected_path_history();
        let enqueue_us = started.elapsed().as_micros();
        let mut max_poll_us = 0;
        while !state.project_tree_state.history_loaded {
            let poll_started = Instant::now();
            state.poll_project_history();
            max_poll_us = max_poll_us.max(poll_started.elapsed().as_micros());
            assert!(started.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(10));
        }
        eprintln!("async_history_enqueue_us={enqueue_us} async_history_max_poll_us={max_poll_us} completion_ms={} commits={}", started.elapsed().as_millis(), state.project_tree_state.history.len());
        assert!(!state.project_history_pending());
    }

    fn controlled_loader(state: &mut AppState) -> (Receiver<ProjectHistoryRequest>, Sender<()>) {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        state.project_history_loader = Some(
            ProjectHistoryLoader::with_loader(move |request| {
                started_tx.send(request.clone()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                super::super::project_history_loader::load_history(request)
            })
            .unwrap(),
        );
        (started_rx, release_tx)
    }

    fn poll_until_loaded(state: &mut AppState) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !state.project_tree_state.history_loaded {
            state.poll_project_history();
            assert!(Instant::now() < deadline, "historique asynchrone attendu");
            std::thread::yield_now();
        }
    }

    fn poll_until_next_request(
        state: &mut AppState,
        started: &Receiver<ProjectHistoryRequest>,
    ) -> ProjectHistoryRequest {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            state.poll_project_history();
            if let Ok(request) = started.try_recv() {
                return request;
            }
            assert!(Instant::now() < deadline, "nouvelle demande attendue");
            std::thread::yield_now();
        }
    }

    #[test]
    fn invalidation_during_loading_replaces_the_request_and_defers_details() {
        let (_temp, mut state) = async_state();
        let (started, release) = controlled_loader(&mut state);
        state.refresh_selected_path_history();
        let original = started.recv_timeout(Duration::from_secs(2)).unwrap();
        state.project_tree_state.invalidate_path_history();
        state.project_tree_state.focus = ProjectTreeFocus::Diff;
        state.ensure_project_tree_focus_loaded();
        assert!(state.project_tree_state.history_loading);
        assert!(!state.project_tree_state.commit_details_loaded);
        assert!(!state.project_tree_state.diff_loaded);
        release.send(()).unwrap();
        let latest = started.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(original.generation, latest.generation);
        assert!(!state.project_tree_state.history_loaded);
        release.send(()).unwrap();
        poll_until_loaded(&mut state);
        assert!(state.project_tree_state.commit_details_loaded);
        assert!(state.project_tree_state.diff_loaded);
        assert!(!state.project_tree_state.history_loading);
    }

    #[test]
    fn switching_repository_discards_the_previous_history() {
        let (_temp, mut state) = async_state();
        let (started, release) = controlled_loader(&mut state);
        state.refresh_selected_path_history();
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let (other_temp, other_repo) = create_test_repo();
        let expected = commit_file(&other_repo, "file.txt", "other", "other repository");
        state.repo = GitRepo { repo: other_repo };
        state.repo_path = other_temp.path().display().to_string();
        release.send(()).unwrap();
        let next = poll_until_next_request(&mut state, &started);
        assert_eq!(next.repo_path, state.repo_path);
        assert!(!state.project_tree_state.history_loaded);
        release.send(()).unwrap();
        poll_until_loaded(&mut state);
        assert_eq!(state.project_tree_state.history.items()[0].oid, expected);
    }

    #[test]
    fn changed_head_is_reloaded_even_before_the_watcher_invalidates_it() {
        let (_temp, mut state) = async_state();
        let (started, release) = controlled_loader(&mut state);
        state.refresh_selected_path_history();
        let original = started.recv_timeout(Duration::from_secs(2)).unwrap();
        let expected = commit_file(&state.repo.repo, "file.txt", "new", "new head");
        release.send(()).unwrap();
        let next = poll_until_next_request(&mut state, &started);
        assert_ne!(next.head, original.head);
        assert!(!state.project_tree_state.history_loaded);
        release.send(()).unwrap();
        poll_until_loaded(&mut state);
        assert_eq!(state.project_tree_state.history.items()[0].oid, expected);
    }

    #[test]
    fn comparison_updates_cannot_apply_an_old_target() {
        let (_temp, mut state) = async_state();
        let head = state
            .repo
            .repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id();
        state
            .repo
            .repo
            .reference("refs/heads/target", head, true, "test")
            .unwrap();
        state
            .project_tree_state
            .start_comparison("main".into(), "target".into());
        let (started, release) = controlled_loader(&mut state);
        state.refresh_selected_path_history();
        let original = started.recv_timeout(Duration::from_secs(2)).unwrap();
        let target = commit_file(&state.repo.repo, "file.txt", "target", "target");
        state
            .repo
            .repo
            .reference("refs/heads/target", target, true, "test")
            .unwrap();
        state
            .repo
            .repo
            .reference("refs/heads/main", head, true, "test")
            .unwrap();
        release.send(()).unwrap();
        let next = poll_until_next_request(&mut state, &started);
        assert_eq!(next.head, original.head);
        assert_ne!(next.comparison, original.comparison);
        assert!(!state.project_tree_state.history_loaded);
        release.send(()).unwrap();
        poll_until_loaded(&mut state);
        assert_eq!(state.project_tree_state.history.items()[0].oid, target);
    }
}
