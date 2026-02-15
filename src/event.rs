use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;

use crate::error::Result;
use crate::state::{AppAction, AppState};
use crate::ui;

/// Gestionnaire de la boucle événementielle.
pub struct EventHandler {
    state: AppState,
}

impl EventHandler {
    /// Crée un nouveau gestionnaire d'événements.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Lance la boucle événementielle principale.
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Rafraîchissement initial si nécessaire
        if self.state.dirty {
            self.refresh()?;
        }

        loop {
            // Render.
            terminal.draw(|frame| {
                ui::render(frame, &self.state);
            })?;

            // Input avec timeout adaptatif (plus long quand aucune animation)
            let timeout_ms = if self.state.flash_message.is_some() {
                100 // 100ms quand il y a un flash message actif
            } else {
                250 // 250ms sinon pour réduire l'utilisation CPU
            };

            if let Some(action) = ui::input::handle_input_with_timeout(&self.state, timeout_ms)? {
                self.apply_action(action)?;
            }

            if self.state.should_quit {
                break;
            }

            // Vérifier si le message flash a expiré.
            let had_flash = self.state.flash_message.is_some();
            self.state.check_flash_expired();

            // Rafraîchissement conditionnel : seulement si dirty
            if self.state.dirty {
                self.refresh()?;
            }
        }
        Ok(())
    }

    /// Applique une action à l'état de l'application.
    fn apply_action(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Quit => {
                self.state.should_quit = true;
            }
            AppAction::MoveUp => self.handle_move_up()?,
            AppAction::MoveDown => self.handle_move_down()?,
            AppAction::PageUp => self.handle_page_up()?,
            AppAction::PageDown => self.handle_page_down()?,
            AppAction::GoTop => self.handle_go_top()?,
            AppAction::GoBottom => self.handle_go_bottom()?,
            AppAction::Select => {
                // Pour l'instant, Select ne fait rien de spécial.
            }
            AppAction::Refresh => self.refresh()?,
            AppAction::ToggleHelp => self.handle_toggle_help(),
            AppAction::SwitchBottomMode => self.handle_switch_bottom_mode(),
            AppAction::BranchList => self.handle_branch_list()?,
            AppAction::CloseBranchPanel => {
                self.state.show_branch_panel = false;
            }
            AppAction::BranchCheckout => self.handle_branch_checkout()?,
            AppAction::BranchCreate => {
                // TODO: implémenter le prompt pour créer une branche
            }
            AppAction::BranchDelete => {
                // TODO: implémenter la confirmation et suppression
            }
            AppAction::FileUp => self.handle_file_up(),
            AppAction::FileDown => self.handle_file_down(),
            AppAction::DiffScrollUp => self.handle_diff_scroll_up(),
            AppAction::DiffScrollDown => self.handle_diff_scroll_down(),
            AppAction::SwitchToGraph => self.handle_switch_to_graph()?,
            AppAction::SwitchToStaging => self.handle_switch_to_staging()?,
            AppAction::SwitchToBranches => self.handle_switch_to_branches()?,
            AppAction::StageFile => self.handle_stage_file()?,
            AppAction::UnstageFile => self.handle_unstage_file()?,
            AppAction::StageAll => self.handle_stage_all()?,
            AppAction::UnstageAll => self.handle_unstage_all()?,
            AppAction::SwitchStagingFocus => self.handle_switch_staging_focus(),
            AppAction::StartCommitMessage => self.handle_start_commit_message(),
            AppAction::ConfirmCommit => self.handle_confirm_commit()?,
            AppAction::CancelCommitMessage => self.handle_cancel_commit_message(),
            AppAction::InsertChar(c) => self.handle_insert_char(c),
            AppAction::DeleteChar => self.handle_delete_char(),
            AppAction::MoveCursorLeft => self.handle_move_cursor_left(),
            AppAction::MoveCursorRight => self.handle_move_cursor_right(),
            AppAction::NextSection => self.handle_next_section(),
            AppAction::PrevSection => self.handle_prev_section(),
            AppAction::BranchRename => self.handle_branch_rename(),
            AppAction::ToggleRemoteBranches => self.handle_toggle_remote_branches(),
            AppAction::WorktreeCreate => self.handle_worktree_create(),
            AppAction::WorktreeRemove => self.handle_worktree_remove()?,
            AppAction::StashApply => self.handle_stash_apply()?,
            AppAction::StashPop => self.handle_stash_pop()?,
            AppAction::StashDrop => self.handle_stash_drop()?,
            AppAction::StashSave => self.handle_stash_save(),
            AppAction::ConfirmInput => self.handle_confirm_input()?,
            AppAction::CancelInput => self.handle_cancel_input(),
            AppAction::CommitPrompt | AppAction::StashPrompt | AppAction::MergePrompt => {
                // TODO: implémenter les modales/prompts interactifs.
            }
        }
        Ok(())
    }

    // Navigation handlers
    fn handle_move_up(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.show_branch_panel {
            if self.state.branch_selected > 0 {
                self.state.branch_selected -= 1;
            }
        } else if self.state.view_mode == ViewMode::Staging {
            self.handle_staging_navigation(-1);
        } else if self.state.view_mode == ViewMode::Branches {
            self.handle_branches_navigation(-1);
        } else if self.state.selected_index > 0 {
            self.state.selected_index -= 1;
            self.state
                .graph_state
                .select(Some(self.state.selected_index * 2));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_move_down(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.show_branch_panel {
            if self.state.branch_selected + 1 < self.state.branches.len() {
                self.state.branch_selected += 1;
            }
        } else if self.state.view_mode == ViewMode::Staging {
            self.handle_staging_navigation(1);
        } else if self.state.view_mode == ViewMode::Branches {
            self.handle_branches_navigation(1);
        } else if self.state.selected_index + 1 < self.state.graph.len() {
            self.state.selected_index += 1;
            self.state
                .graph_state
                .select(Some(self.state.selected_index * 2));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_page_up(&mut self) -> Result<()> {
        if !self.state.show_branch_panel && !self.state.graph.is_empty() {
            let page_size = 10;
            self.state.selected_index = self.state.selected_index.saturating_sub(page_size);
            self.state
                .graph_state
                .select(Some(self.state.selected_index * 2));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_page_down(&mut self) -> Result<()> {
        if !self.state.show_branch_panel && !self.state.graph.is_empty() {
            let page_size = 10;
            self.state.selected_index =
                (self.state.selected_index + page_size).min(self.state.graph.len() - 1);
            self.state
                .graph_state
                .select(Some(self.state.selected_index * 2));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_go_top(&mut self) -> Result<()> {
        if !self.state.show_branch_panel && !self.state.graph.is_empty() {
            self.state.selected_index = 0;
            self.state.graph_state.select(Some(0));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_go_bottom(&mut self) -> Result<()> {
        if !self.state.show_branch_panel && !self.state.graph.is_empty() {
            self.state.selected_index = self.state.graph.len() - 1;
            self.state
                .graph_state
                .select(Some(self.state.selected_index * 2));
            self.update_commit_files();
        }
        Ok(())
    }

    // View mode handlers
    fn handle_toggle_help(&mut self) {
        use crate::state::ViewMode;
        self.state.view_mode = if self.state.view_mode == ViewMode::Help {
            ViewMode::Graph
        } else {
            ViewMode::Help
        };
    }

    fn handle_switch_bottom_mode(&mut self) {
        use crate::state::FocusPanel;
        self.state.focus = match self.state.focus {
            FocusPanel::Graph => FocusPanel::Files,
            FocusPanel::Files => FocusPanel::Detail,
            FocusPanel::Detail => FocusPanel::Graph,
        };
    }

    fn handle_switch_to_graph(&mut self) -> Result<()> {
        use crate::state::ViewMode;
        self.state.view_mode = ViewMode::Graph;
        self.refresh()
    }

    fn handle_switch_to_staging(&mut self) -> Result<()> {
        use crate::state::ViewMode;
        self.state.view_mode = ViewMode::Staging;
        self.refresh_staging()
    }

    fn handle_switch_to_branches(&mut self) -> Result<()> {
        use crate::state::ViewMode;
        self.state.view_mode = ViewMode::Branches;
        self.refresh_branches_view()
    }

    // Branch panel handlers
    fn handle_branch_list(&mut self) -> Result<()> {
        if self.state.show_branch_panel {
            self.state.show_branch_panel = false;
        } else {
            self.state.branches = self.state.repo.branches().unwrap_or_default();
            self.state.branch_selected = 0;
            self.state.show_branch_panel = true;
        }
        Ok(())
    }

    fn handle_branch_checkout(&mut self) -> Result<()> {
        if self.state.show_branch_panel {
            if let Some(branch) = self.state.branches.get(self.state.branch_selected).cloned() {
                if let Err(e) = self.state.repo.checkout_branch(&branch.name) {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    self.state.show_branch_panel = false;
                    self.state.mark_dirty(); // Marquer comme modifié - checkout
                    self.refresh()?;
                    self.state
                        .set_flash_message(format!("Checkout sur '{}'", branch.name));
                }
            }
        }
        Ok(())
    }

    // File navigation handlers
    fn handle_file_up(&mut self) {
        if self.state.focus == crate::state::FocusPanel::Files
            && !self.state.commit_files.is_empty()
        {
            if self.state.file_selected_index > 0 {
                self.state.file_selected_index -= 1;
                self.load_selected_file_diff();
            }
        }
    }

    fn handle_file_down(&mut self) {
        if self.state.focus == crate::state::FocusPanel::Files
            && !self.state.commit_files.is_empty()
        {
            if self.state.file_selected_index + 1 < self.state.commit_files.len() {
                self.state.file_selected_index += 1;
                self.load_selected_file_diff();
            }
        }
    }

    // Diff scroll handlers
    fn handle_diff_scroll_up(&mut self) {
        use crate::state::{FocusPanel, ViewMode};

        if self.state.focus == FocusPanel::Detail && self.state.diff_scroll_offset > 0 {
            self.state.diff_scroll_offset -= 1;
        } else if self.state.view_mode == ViewMode::Staging
            && self.state.staging_state.diff_scroll > 0
        {
            self.state.staging_state.diff_scroll -= 1;
        }
    }

    fn handle_diff_scroll_down(&mut self) {
        use crate::state::{FocusPanel, ViewMode};

        if self.state.focus == FocusPanel::Detail {
            self.state.diff_scroll_offset += 1;
        } else if self.state.view_mode == ViewMode::Staging {
            self.state.staging_state.diff_scroll += 1;
        }
    }

    // Staging handlers
    fn handle_stage_file(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            if let Some(file) = self
                .state
                .staging_state
                .unstaged_files
                .get(self.state.staging_state.unstaged_selected)
            {
                crate::git::commit::stage_file(&self.state.repo.repo, &file.path)?;
                self.state.mark_dirty(); // Marquer comme modifié
                self.refresh_staging()?;
            }
        }
        Ok(())
    }

    fn handle_unstage_file(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            if let Some(file) = self
                .state
                .staging_state
                .staged_files
                .get(self.state.staging_state.staged_selected)
            {
                crate::git::commit::unstage_file(&self.state.repo.repo, &file.path)?;
                self.state.mark_dirty(); // Marquer comme modifié
                self.refresh_staging()?;
            }
        }
        Ok(())
    }

    fn handle_stage_all(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            crate::git::commit::stage_all(&self.state.repo.repo)?;
            self.state.mark_dirty(); // Marquer comme modifié
            self.refresh_staging()?;
        }
        Ok(())
    }

    fn handle_unstage_all(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            crate::git::commit::unstage_all(&self.state.repo.repo)?;
            self.state.mark_dirty(); // Marquer comme modifié
            self.refresh_staging()?;
        }
        Ok(())
    }

    fn handle_switch_staging_focus(&mut self) {
        use crate::state::{StagingFocus, ViewMode};

        if self.state.view_mode == ViewMode::Staging {
            self.state.staging_state.focus = match self.state.staging_state.focus {
                StagingFocus::Unstaged => StagingFocus::Staged,
                StagingFocus::Staged => StagingFocus::Diff,
                StagingFocus::Diff => StagingFocus::Unstaged,
                StagingFocus::CommitMessage => StagingFocus::Unstaged,
            };
            self.load_staging_diff();
        }
    }

    fn handle_start_commit_message(&mut self) {
        use crate::state::{StagingFocus, ViewMode};

        if self.state.view_mode == ViewMode::Staging {
            self.state.staging_state.is_committing = true;
            self.state.staging_state.focus = StagingFocus::CommitMessage;
        }
    }

    fn handle_confirm_commit(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging
            && !self.state.staging_state.commit_message.is_empty()
            && !self.state.staging_state.staged_files.is_empty()
        {
            crate::git::commit::create_commit(
                &self.state.repo.repo,
                &self.state.staging_state.commit_message,
            )?;
            self.state.staging_state.commit_message.clear();
            self.state.staging_state.cursor_position = 0;
            self.state.staging_state.is_committing = false;
            self.state.mark_dirty(); // Marquer comme modifié - nouveau commit
            self.refresh_staging()?;
            self.state
                .set_flash_message("Commit créé avec succès".into());
        }
        Ok(())
    }

    fn handle_cancel_commit_message(&mut self) {
        use crate::state::{StagingFocus, ViewMode};

        if self.state.view_mode == ViewMode::Staging {
            self.state.staging_state.is_committing = false;
            self.state.staging_state.focus = StagingFocus::Unstaged;
        }
    }

    // Input handlers
    fn handle_insert_char(&mut self, c: char) {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging && self.state.staging_state.is_committing {
            // Utiliser char_indices pour trouver la position d'octet correcte
            let byte_pos = self.char_to_byte_position(
                &self.state.staging_state.commit_message,
                self.state.staging_state.cursor_position,
            );
            self.state.staging_state.commit_message.insert(byte_pos, c);
            self.state.staging_state.cursor_position += 1;
        }
    }

    fn handle_delete_char(&mut self) {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging
            && self.state.staging_state.is_committing
            && self.state.staging_state.cursor_position > 0
        {
            self.state.staging_state.cursor_position -= 1;
            let byte_pos = self.char_to_byte_position(
                &self.state.staging_state.commit_message,
                self.state.staging_state.cursor_position,
            );
            let end_byte_pos = self.char_to_byte_position(
                &self.state.staging_state.commit_message,
                self.state.staging_state.cursor_position + 1,
            );
            self.state
                .staging_state
                .commit_message
                .drain(byte_pos..end_byte_pos);
        }
    }

    fn handle_move_cursor_left(&mut self) {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging
            && self.state.staging_state.is_committing
            && self.state.staging_state.cursor_position > 0
        {
            self.state.staging_state.cursor_position -= 1;
        }
    }

    fn handle_move_cursor_right(&mut self) {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging
            && self.state.staging_state.is_committing
            && self.state.staging_state.cursor_position
                < self.state.staging_state.commit_message.chars().count()
        {
            self.state.staging_state.cursor_position += 1;
        }
    }

    /// Convertit une position de caractère en position d'octet pour gérer Unicode correctement.
    fn char_to_byte_position(&self, s: &str, char_pos: usize) -> usize {
        s.char_indices()
            .nth(char_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len())
    }

    // Branches view handlers
    fn handle_next_section(&mut self) {
        use crate::state::{BranchesSection, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.section = match self.state.branches_view_state.section {
                BranchesSection::Branches => BranchesSection::Worktrees,
                BranchesSection::Worktrees => BranchesSection::Stashes,
                BranchesSection::Stashes => BranchesSection::Branches,
            };
        }
    }

    fn handle_prev_section(&mut self) {
        use crate::state::{BranchesSection, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.section = match self.state.branches_view_state.section {
                BranchesSection::Branches => BranchesSection::Stashes,
                BranchesSection::Worktrees => BranchesSection::Branches,
                BranchesSection::Stashes => BranchesSection::Worktrees,
            };
        }
    }

    fn handle_branch_rename(&mut self) {
        use crate::state::{BranchesFocus, InputAction, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.focus = BranchesFocus::Input;
            self.state.branches_view_state.input_action = Some(InputAction::RenameBranch);
            self.state.branches_view_state.input_text.clear();
            self.state.branches_view_state.input_cursor = 0;
        }
    }

    fn handle_toggle_remote_branches(&mut self) {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.show_remote =
                !self.state.branches_view_state.show_remote;
        }
    }

    fn handle_worktree_create(&mut self) {
        use crate::state::{BranchesFocus, InputAction, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.focus = BranchesFocus::Input;
            self.state.branches_view_state.input_action = Some(InputAction::CreateWorktree);
            self.state.branches_view_state.input_text.clear();
            self.state.branches_view_state.input_cursor = 0;
        }
    }

    fn handle_worktree_remove(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            if let Some(worktree) = self
                .state
                .branches_view_state
                .worktrees
                .get(self.state.branches_view_state.worktree_selected)
            {
                if !worktree.is_main {
                    let name = worktree.name.clone();
                    if let Err(e) = self.state.repo.remove_worktree(&name) {
                        self.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        self.state
                            .set_flash_message(format!("Worktree '{}' supprimé", name));
                        self.state.mark_dirty(); // Marquer comme modifié
                        self.refresh_branches_view()?;
                    }
                } else {
                    self.state
                        .set_flash_message("Impossible de supprimer le worktree principal".into());
                }
            }
        }
        Ok(())
    }

    fn handle_stash_apply(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            if let Some(stash) = self
                .state
                .branches_view_state
                .stashes
                .get(self.state.branches_view_state.stash_selected)
            {
                let idx = stash.index;
                if let Err(e) = crate::git::stash::apply_stash(&mut self.state.repo.repo, idx) {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    self.state
                        .set_flash_message(format!("Stash @{{{}}} appliqué", idx));
                    self.state.mark_dirty(); // Marquer comme modifié
                    self.refresh_branches_view()?;
                }
            }
        }
        Ok(())
    }

    fn handle_stash_pop(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            if let Some(stash) = self
                .state
                .branches_view_state
                .stashes
                .get(self.state.branches_view_state.stash_selected)
            {
                let idx = stash.index;
                if let Err(e) = crate::git::stash::pop_stash(&mut self.state.repo.repo, idx) {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    self.state
                        .set_flash_message(format!("Stash @{{{}}} appliqué et supprimé", idx));
                    self.state.mark_dirty(); // Marquer comme modifié
                    self.refresh_branches_view()?;
                }
            }
        }
        Ok(())
    }

    fn handle_stash_drop(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            if let Some(stash) = self
                .state
                .branches_view_state
                .stashes
                .get(self.state.branches_view_state.stash_selected)
            {
                let idx = stash.index;
                if let Err(e) = crate::git::stash::drop_stash(&mut self.state.repo.repo, idx) {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    self.state
                        .set_flash_message(format!("Stash @{{{}}} supprimé", idx));
                    self.state.mark_dirty(); // Marquer comme modifié - stash modifié
                    self.refresh_branches_view()?;
                }
            }
        }
        Ok(())
    }

    fn handle_stash_save(&mut self) {
        use crate::state::{BranchesFocus, InputAction, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.focus = BranchesFocus::Input;
            self.state.branches_view_state.input_action = Some(InputAction::SaveStash);
            self.state.branches_view_state.input_text.clear();
            self.state.branches_view_state.input_cursor = 0;
        }
    }

    fn handle_confirm_input(&mut self) -> Result<()> {
        use crate::state::{BranchesFocus, InputAction, ViewMode};

        if self.state.view_mode == ViewMode::Branches
            && self.state.branches_view_state.focus == BranchesFocus::Input
        {
            match self.state.branches_view_state.input_action.take() {
                Some(InputAction::CreateBranch) => {
                    let name = self.state.branches_view_state.input_text.clone();
                    if !name.is_empty() {
                        if let Err(e) =
                            crate::git::branch::create_branch(&self.state.repo.repo, &name)
                        {
                            self.state.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.state
                                .set_flash_message(format!("Branche '{}' créée", name));
                            self.state.mark_dirty(); // Marquer comme modifié
                            self.refresh_branches_view()?;
                        }
                    }
                }
                Some(InputAction::RenameBranch) => {
                    if let Some(branch) = self
                        .state
                        .branches_view_state
                        .local_branches
                        .get(self.state.branches_view_state.branch_selected)
                    {
                        let old_name = branch.name.clone();
                        let new_name = self.state.branches_view_state.input_text.clone();
                        if !new_name.is_empty() && new_name != old_name {
                            if let Err(e) = crate::git::branch::rename_branch(
                                &self.state.repo.repo,
                                &old_name,
                                &new_name,
                            ) {
                                self.state.set_flash_message(format!("Erreur: {}", e));
                            } else {
                                self.state.set_flash_message(format!(
                                    "Branche '{}' renommée en '{}'",
                                    old_name, new_name
                                ));
                                self.state.mark_dirty(); // Marquer comme modifié
                                self.refresh_branches_view()?;
                            }
                        }
                    }
                }
                Some(InputAction::CreateWorktree) => {
                    let input = self.state.branches_view_state.input_text.clone();
                    // Format attendu: "nom chemin [branche]"
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[0];
                        let path = parts[1];
                        let branch = parts.get(2).map(|s| *s);
                        if let Err(e) = self.state.repo.create_worktree(name, path, branch) {
                            self.state.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.state
                                .set_flash_message(format!("Worktree '{}' créé", name));
                            self.state.mark_dirty(); // Marquer comme modifié
                            self.refresh_branches_view()?;
                        }
                    } else {
                        self.state
                            .set_flash_message("Format: nom chemin [branche]".into());
                    }
                }
                Some(InputAction::SaveStash) => {
                    let msg = self.state.branches_view_state.input_text.clone();
                    let msg_opt = if msg.is_empty() {
                        None
                    } else {
                        Some(msg.as_str())
                    };
                    if let Err(e) =
                        crate::git::stash::save_stash(&mut self.state.repo.repo, msg_opt)
                    {
                        self.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        self.state.set_flash_message("Stash sauvegardé".into());
                        self.state.mark_dirty(); // Marquer comme modifié
                        self.refresh_branches_view()?;
                    }
                }
                _ => {}
            }
            self.state.branches_view_state.focus = BranchesFocus::List;
            self.state.branches_view_state.input_text.clear();
            self.state.branches_view_state.input_cursor = 0;
        }
        Ok(())
    }

    fn handle_cancel_input(&mut self) {
        use crate::state::{BranchesFocus, ViewMode};

        if self.state.view_mode == ViewMode::Branches {
            self.state.branches_view_state.focus = BranchesFocus::List;
            self.state.branches_view_state.input_action = None;
            self.state.branches_view_state.input_text.clear();
            self.state.branches_view_state.input_cursor = 0;
        }
    }

    // Helper methods
    fn update_commit_files(&mut self) {
        if let Some(row) = self.state.graph.get(self.state.selected_index) {
            self.state.commit_files = self
                .state
                .repo
                .commit_diff(row.node.oid)
                .unwrap_or_default();
        } else {
            self.state.commit_files.clear();
        }
        // Réinitialiser le diff sélectionné quand on change de commit.
        self.state.file_selected_index = 0;
        self.state.selected_file_diff = None;
        self.state.diff_scroll_offset = 0;
    }

    fn load_selected_file_diff(&mut self) {
        if let Some(file) = self.state.commit_files.get(self.state.file_selected_index) {
            if let Some(row) = self.state.graph.get(self.state.selected_index) {
                let cache_key = (row.node.oid, file.path.clone());

                // Essayer de récupérer du cache
                if let Some(cached_diff) = self.state.diff_cache.get(&cache_key) {
                    self.state.selected_file_diff = Some(cached_diff.clone());
                } else {
                    // Calculer et mettre en cache
                    match self.state.repo.file_diff(row.node.oid, &file.path) {
                        Ok(diff) => {
                            self.state.diff_cache.insert(cache_key, diff.clone());
                            self.state.selected_file_diff = Some(diff);
                        }
                        Err(_) => {
                            self.state.selected_file_diff = None;
                        }
                    }
                }
            }
        } else {
            self.state.selected_file_diff = None;
        }
        self.state.diff_scroll_offset = 0;
    }

    fn load_staging_diff(&mut self) {
        use crate::state::StagingFocus;

        let selected_file = match self.state.staging_state.focus {
            StagingFocus::Unstaged => self
                .state
                .staging_state
                .unstaged_files
                .get(self.state.staging_state.unstaged_selected),
            StagingFocus::Staged => self
                .state
                .staging_state
                .staged_files
                .get(self.state.staging_state.staged_selected),
            _ => None,
        };

        if let Some(file) = selected_file {
            // Pour le working directory, on utilise Oid::zero() comme clé spéciale
            let cache_key = (git2::Oid::zero(), file.path.clone());

            // Essayer de récupérer du cache
            if let Some(cached_diff) = self.state.diff_cache.get(&cache_key) {
                self.state.staging_state.current_diff = Some(cached_diff.clone());
            } else {
                // Calculer et mettre en cache
                match crate::git::diff::working_dir_file_diff(&self.state.repo.repo, &file.path) {
                    Ok(diff) => {
                        self.state.diff_cache.insert(cache_key, diff.clone());
                        self.state.staging_state.current_diff = Some(diff);
                    }
                    Err(_) => {
                        self.state.staging_state.current_diff = None;
                    }
                }
            }
        } else {
            self.state.staging_state.current_diff = None;
        }
        self.state.staging_state.diff_scroll = 0;
    }

    fn handle_staging_navigation(&mut self, direction: i32) {
        use crate::state::StagingFocus;

        match self.state.staging_state.focus {
            StagingFocus::Unstaged => {
                let max = self.state.staging_state.unstaged_files.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.state.staging_state.unstaged_selected + 1).min(max - 1)
                    } else {
                        self.state.staging_state.unstaged_selected.saturating_sub(1)
                    };
                    self.state.staging_state.unstaged_selected = new_idx;
                    self.load_staging_diff();
                }
            }
            StagingFocus::Staged => {
                let max = self.state.staging_state.staged_files.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.state.staging_state.staged_selected + 1).min(max - 1)
                    } else {
                        self.state.staging_state.staged_selected.saturating_sub(1)
                    };
                    self.state.staging_state.staged_selected = new_idx;
                    self.load_staging_diff();
                }
            }
            StagingFocus::Diff => {
                if direction > 0 {
                    self.state.staging_state.diff_scroll += 1;
                } else if self.state.staging_state.diff_scroll > 0 {
                    self.state.staging_state.diff_scroll -= 1;
                }
            }
            _ => {}
        }
    }

    fn handle_branches_navigation(&mut self, direction: i32) {
        use crate::state::BranchesSection;

        match self.state.branches_view_state.section {
            BranchesSection::Branches => {
                let max = self.state.branches_view_state.local_branches.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.state.branches_view_state.branch_selected + 1).min(max - 1)
                    } else {
                        self.state
                            .branches_view_state
                            .branch_selected
                            .saturating_sub(1)
                    };
                    self.state.branches_view_state.branch_selected = new_idx;
                }
            }
            BranchesSection::Worktrees => {
                let max = self.state.branches_view_state.worktrees.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.state.branches_view_state.worktree_selected + 1).min(max - 1)
                    } else {
                        self.state
                            .branches_view_state
                            .worktree_selected
                            .saturating_sub(1)
                    };
                    self.state.branches_view_state.worktree_selected = new_idx;
                }
            }
            BranchesSection::Stashes => {
                let max = self.state.branches_view_state.stashes.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.state.branches_view_state.stash_selected + 1).min(max - 1)
                    } else {
                        self.state
                            .branches_view_state
                            .stash_selected
                            .saturating_sub(1)
                    };
                    self.state.branches_view_state.stash_selected = new_idx;
                }
            }
        }
    }

    fn refresh(&mut self) -> Result<()> {
        use crate::state::MAX_COMMITS;

        self.state.current_branch = self.state.repo.current_branch().ok();
        self.state.graph = self.state.repo.build_graph(MAX_COMMITS).unwrap_or_default();
        self.state.status_entries = self.state.repo.status().unwrap_or_default();

        // Réajuster la sélection si nécessaire.
        if self.state.selected_index >= self.state.graph.len() && !self.state.graph.is_empty() {
            self.state.selected_index = self.state.graph.len() - 1;
        }

        // Charger les fichiers du commit sélectionné.
        self.update_commit_files();

        // Réinitialiser la sélection de fichier.
        self.state.file_selected_index = 0;
        self.state.selected_file_diff = None;
        self.state.diff_scroll_offset = 0;

        // Rafraîchir aussi l'état de staging en passant les status_entries déjà récupérés.
        self.refresh_staging_with_entries(&self.state.status_entries.clone())?;

        // Marquer comme à jour après le rafraîchissement
        self.state.clear_dirty();

        Ok(())
    }

    fn refresh_staging(&mut self) -> Result<()> {
        let all_entries = self.state.repo.status()?;
        self.refresh_staging_with_entries(&all_entries)
    }

    fn refresh_staging_with_entries(
        &mut self,
        all_entries: &[crate::git::repo::StatusEntry],
    ) -> Result<()> {
        self.state.staging_state.staged_files = all_entries
            .iter()
            .filter(|e| e.is_staged())
            .cloned()
            .collect();

        self.state.staging_state.unstaged_files = all_entries
            .iter()
            .filter(|e| e.is_unstaged())
            .cloned()
            .collect();

        // Réajuster les sélections.
        if self.state.staging_state.unstaged_selected
            >= self.state.staging_state.unstaged_files.len()
        {
            self.state.staging_state.unstaged_selected = self
                .state
                .staging_state
                .unstaged_files
                .len()
                .saturating_sub(1);
        }
        if self.state.staging_state.staged_selected >= self.state.staging_state.staged_files.len() {
            self.state.staging_state.staged_selected = self
                .state
                .staging_state
                .staged_files
                .len()
                .saturating_sub(1);
        }

        // Charger le diff du fichier survolé.
        self.load_staging_diff();

        Ok(())
    }

    fn refresh_branches_view(&mut self) -> Result<()> {
        let (local_branches, remote_branches) =
            crate::git::branch::list_all_branches(&self.state.repo.repo)?;

        self.state.branches_view_state.local_branches = local_branches;
        self.state.branches_view_state.remote_branches = remote_branches;
        self.state.branches_view_state.worktrees = self.state.repo.worktrees().unwrap_or_default();
        self.state.branches_view_state.stashes = self.state.repo.stashes().unwrap_or_default();

        // Réajuster les sélections.
        if self.state.branches_view_state.branch_selected
            >= self.state.branches_view_state.local_branches.len()
        {
            self.state.branches_view_state.branch_selected = self
                .state
                .branches_view_state
                .local_branches
                .len()
                .saturating_sub(1);
        }
        if self.state.branches_view_state.worktree_selected
            >= self.state.branches_view_state.worktrees.len()
        {
            self.state.branches_view_state.worktree_selected = self
                .state
                .branches_view_state
                .worktrees
                .len()
                .saturating_sub(1);
        }
        if self.state.branches_view_state.stash_selected
            >= self.state.branches_view_state.stashes.len()
        {
            self.state.branches_view_state.stash_selected = self
                .state
                .branches_view_state
                .stashes
                .len()
                .saturating_sub(1);
        }

        Ok(())
    }
}
