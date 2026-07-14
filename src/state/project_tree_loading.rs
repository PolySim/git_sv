//! Chargement paresseux des panneaux de la vue arborescence.

use super::{AppState, ProjectTreeFocus, MAX_TOTAL_COMMITS};

impl AppState {
    /// Rafraîchit l'arborescence sans charger les panneaux dépendants.
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
            return;
        };

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

    /// Recharge les fichiers touchés par le commit d'historique sélectionné.
    pub fn refresh_selected_history_commit_details(&mut self) {
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
        if let Some(diff) = self.diff_cache.get(&cache_key).cloned() {
            self.project_tree_state.set_selected_diff(Some(diff));
            return;
        }

        match self.repo.file_diff(commit_oid, &file_path) {
            Ok(diff) => {
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
                if !self.project_tree_state.commit_details_loaded {
                    self.refresh_selected_history_commit_details();
                }
            }
            ProjectTreeFocus::Diff => {
                if !self.project_tree_state.history_loaded {
                    self.refresh_selected_path_history();
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
