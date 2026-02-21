use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;

use crate::error::{GitSvError, Result};
use crate::state::{AppAction, AppState};
use crate::ui;
use crate::ui::confirm_dialog::ConfirmAction;

/// Copie le texte dans le clipboard système.
fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| GitSvError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text)
        .map_err(|e| GitSvError::Clipboard(e.to_string()))?;
    Ok(())
}

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
            AppAction::BranchCreate => self.handle_branch_create()?,
            AppAction::BranchDelete => self.handle_branch_delete_trigger()?,
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
            AppAction::ConfirmAction => self.handle_confirm_action()?,
            AppAction::CancelAction => self.handle_cancel_action(),
            AppAction::CommitPrompt => self.handle_commit_prompt()?,
            AppAction::StashPrompt => self.handle_stash_prompt()?,
            AppAction::MergePrompt => self.handle_merge_prompt()?,
            AppAction::GitPush => self.handle_git_push()?,
            AppAction::GitPull => self.handle_git_pull()?,
            AppAction::GitFetch => self.handle_git_fetch()?,
            AppAction::OpenSearch => self.handle_open_search(),
            AppAction::CloseSearch => self.handle_close_search(),
            AppAction::ChangeSearchType => self.handle_change_search_type(),
            AppAction::NextSearchResult => self.handle_next_search_result(),
            AppAction::PrevSearchResult => self.handle_prev_search_result(),
            AppAction::DiscardFile => self.handle_discard_file()?,
            AppAction::DiscardAll => self.handle_discard_all()?,
            AppAction::OpenBlame => self.handle_open_blame()?,
            AppAction::CloseBlame => self.handle_close_blame(),
            AppAction::JumpToBlameCommit => self.handle_jump_to_blame_commit()?,
            AppAction::CherryPick => self.handle_cherry_pick()?,
            AppAction::AmendCommit => self.handle_amend_commit()?,
            AppAction::MergePickerUp => self.handle_merge_picker_up(),
            AppAction::MergePickerDown => self.handle_merge_picker_down(),
            AppAction::MergePickerConfirm => self.handle_merge_picker_confirm()?,
            AppAction::MergePickerCancel => self.handle_merge_picker_cancel(),
            AppAction::SwitchToConflicts => self.handle_switch_to_conflicts(),
            AppAction::ConflictEnterResolve => self.handle_conflict_enter_resolve()?,
            AppAction::ConflictChooseBoth => self.handle_conflict_choose_both()?,
            AppAction::ConflictFileChooseOurs => self.handle_conflict_file_choose_ours(),
            AppAction::ConflictFileChooseTheirs => self.handle_conflict_file_choose_theirs(),
            AppAction::ConflictNextFile => self.handle_conflict_next_file(),
            AppAction::ConflictPrevFile => self.handle_conflict_prev_file(),
            AppAction::ConflictNextSection => self.handle_conflict_next_section(),
            AppAction::ConflictPrevSection => self.handle_conflict_prev_section(),
            AppAction::ConflictResolveFile => self.handle_conflict_resolve_file()?,
            AppAction::ConflictFinalize => self.handle_conflict_finalize()?,
            AppAction::ConflictAbort => self.handle_conflict_abort()?,
            AppAction::ConflictSetModeFile => self.handle_conflict_set_mode_file()?,
            AppAction::ConflictSetModeBlock => self.handle_conflict_set_mode_block()?,
            AppAction::ConflictSetModeLine => self.handle_conflict_set_mode_line()?,
            AppAction::ConflictToggleLine => self.handle_conflict_toggle_line(),
            AppAction::ConflictLineDown => self.handle_conflict_line_down(),
            AppAction::ConflictLineUp => self.handle_conflict_line_up(),
            AppAction::ConflictSwitchPanelForward => self.handle_conflict_switch_panel_forward(),
            AppAction::ConflictSwitchPanelReverse => self.handle_conflict_switch_panel_reverse(),
            AppAction::ConflictResultScrollDown => self.handle_conflict_result_scroll_down(),
            AppAction::ConflictResultScrollUp => self.handle_conflict_result_scroll_up(),
            AppAction::ConflictValidateMerge => self.handle_conflict_validate_merge()?,
            AppAction::ConflictStartEditing => self.handle_conflict_start_editing(),
            AppAction::ConflictStopEditing => self.handle_conflict_stop_editing(),
            AppAction::ConflictEditInsertChar(c) => self.handle_conflict_edit_insert_char(c),
            AppAction::ConflictEditBackspace => self.handle_conflict_edit_backspace(),
            AppAction::ConflictEditDelete => self.handle_conflict_edit_delete(),
            AppAction::ConflictEditCursorUp => self.handle_conflict_edit_cursor_up(),
            AppAction::ConflictEditCursorDown => self.handle_conflict_edit_cursor_down(),
            AppAction::ConflictEditCursorLeft => self.handle_conflict_edit_cursor_left(),
            AppAction::ConflictEditCursorRight => self.handle_conflict_edit_cursor_right(),
            AppAction::ConflictEditNewline => self.handle_conflict_edit_newline(),
            AppAction::StashSelectedFile => self.handle_stash_selected_file()?,
            AppAction::StashUnstagedFiles => self.handle_stash_unstaged_files()?,
            AppAction::CopyPanelContent => self.handle_copy_panel_content()?,
        }
        Ok(())
    }

    /// Gère la confirmation d'une action destructive.
    fn handle_confirm_action(&mut self) -> Result<()> {
        use crate::ui::confirm_dialog::ConfirmAction;

        if let Some(action) = self.state.pending_confirmation.take() {
            match action {
                ConfirmAction::BranchDelete(name) => {
                    self.execute_branch_delete(&name)?;
                }
                ConfirmAction::WorktreeRemove(name) => {
                    self.execute_worktree_remove(&name)?;
                }
                ConfirmAction::StashDrop(index) => {
                    self.execute_stash_drop(index)?;
                }
                ConfirmAction::DiscardFile(path) => {
                    self.execute_discard_file(&path)?;
                }
                ConfirmAction::DiscardAll => {
                    self.execute_discard_all()?;
                }
                ConfirmAction::CherryPick(oid) => {
                    self.execute_cherry_pick(oid)?;
                }
                ConfirmAction::MergeBranch(source, _) => {
                    // C'est une validation de merge (ConflictValidateMerge)
                    if self.state.view_mode == crate::state::ViewMode::Conflicts {
                        self.execute_conflict_validate_merge(&source)?;
                    } else {
                        self.execute_merge(&source)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Exécute la validation du merge après confirmation.
    fn execute_conflict_validate_merge(&mut self, _message: &str) -> Result<()> {
        use crate::git::conflict::finalize_merge;
        use crate::state::ViewMode;

        if let Some(ref conflicts_state) = self.state.conflicts_state {
            let message = format!("Merge: {}", conflicts_state.operation_description);
            match finalize_merge(&self.state.repo.repo, &message) {
                Ok(()) => {
                    self.state.conflicts_state = None;
                    self.state.view_mode = ViewMode::Graph;
                    self.state
                        .set_flash_message("Merge finalisé avec succès ✓".into());
                    self.state.mark_dirty();
                    self.refresh()?;
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur lors de la finalisation: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Annule l'action en attente de confirmation.
    fn handle_cancel_action(&mut self) {
        self.state.pending_confirmation = None;
    }

    /// Exécute la suppression d'une branche.
    fn execute_branch_delete(&mut self, name: &str) -> Result<()> {
        if let Err(e) = crate::git::branch::delete_branch(&self.state.repo.repo, name) {
            self.state.set_flash_message(format!("Erreur: {}", e));
        } else {
            self.state
                .set_flash_message(format!("Branche '{}' supprimée", name));
            self.state.mark_dirty();
            self.refresh()?;
        }
        Ok(())
    }

    /// Exécute la suppression d'un worktree.
    fn execute_worktree_remove(&mut self, name: &str) -> Result<()> {
        if let Err(e) = self.state.repo.remove_worktree(name) {
            self.state.set_flash_message(format!("Erreur: {}", e));
        } else {
            self.state
                .set_flash_message(format!("Worktree '{}' supprimé", name));
            self.state.mark_dirty();
            self.refresh_branches_view()?;
        }
        Ok(())
    }

    /// Exécute la suppression d'un stash.
    fn execute_stash_drop(&mut self, index: usize) -> Result<()> {
        if let Err(e) = crate::git::stash::drop_stash(&mut self.state.repo.repo, index) {
            self.state.set_flash_message(format!("Erreur: {}", e));
        } else {
            self.state
                .set_flash_message(format!("Stash @{{{}}} supprimé", index));
            self.state.mark_dirty();
            self.refresh_branches_view()?;
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
        } else if self.state.view_mode == ViewMode::Blame {
            self.handle_blame_navigation(-1);
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
        } else if self.state.view_mode == ViewMode::Blame {
            self.handle_blame_navigation(1);
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
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Blame {
            self.handle_blame_navigation(-10);
        } else if !self.state.show_branch_panel && !self.state.graph.is_empty() {
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
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Blame {
            self.handle_blame_navigation(10);
        } else if !self.state.show_branch_panel && !self.state.graph.is_empty() {
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
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Blame {
            if let Some(ref mut blame_state) = self.state.blame_state {
                blame_state.selected_line = 0;
                blame_state.scroll_offset = 0;
            }
        } else if !self.state.show_branch_panel && !self.state.graph.is_empty() {
            self.state.selected_index = 0;
            self.state.graph_state.select(Some(0));
            self.update_commit_files();
        }
        Ok(())
    }

    fn handle_go_bottom(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Blame {
            if let Some(ref mut blame_state) = self.state.blame_state {
                if let Some(ref blame) = blame_state.blame {
                    if !blame.lines.is_empty() {
                        blame_state.selected_line = blame.lines.len() - 1;
                    }
                }
            }
        } else if !self.state.show_branch_panel && !self.state.graph.is_empty() {
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

        // Si on est en mode Help, revenir à la vue précédente
        // Sinon, passer en mode Help
        self.state.view_mode = if self.state.view_mode == ViewMode::Help {
            // Déterminer la vue précédente (si conflits, y retourner)
            if self.state.conflicts_state.is_some() {
                ViewMode::Conflicts
            } else {
                ViewMode::Graph
            }
        } else {
            ViewMode::Help
        };
    }

    fn handle_switch_bottom_mode(&mut self) {
        use crate::state::FocusPanel;

        let previous_focus = self.state.focus;

        self.state.focus = match self.state.focus {
            FocusPanel::Graph => FocusPanel::Files,
            FocusPanel::Files => FocusPanel::Graph,
            FocusPanel::Detail => FocusPanel::Graph,
        };

        // Auto-sélectionner le premier fichier quand on passe au panneau Files
        if previous_focus == FocusPanel::Graph && self.state.focus == FocusPanel::Files {
            if !self.state.commit_files.is_empty() && self.state.selected_file_diff.is_none() {
                self.state.file_selected_index = 0;
                self.load_selected_file_diff();
            }
        }
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
        use crate::state::ViewMode;

        if self.state.show_branch_panel {
            // Panneau legacy (vue Graph, touche 'b')
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
        } else if self.state.view_mode == ViewMode::Branches {
            // Vue Branches dédiée (touche '3')
            if let Some(branch) = self
                .state
                .branches_view_state
                .local_branches
                .get(self.state.branches_view_state.branch_selected)
                .cloned()
            {
                if let Err(e) = self.state.repo.checkout_branch(&branch.name) {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    self.state.mark_dirty(); // Marquer comme modifié - checkout
                    self.refresh()?;
                    self.refresh_branches_view()?; // Rafraîchir la vue Branches
                    self.state
                        .set_flash_message(format!("Checkout sur '{}'", branch.name));
                }
            }
        }
        Ok(())
    }

    // File navigation handlers
    fn handle_file_up(&mut self) {
        use crate::state::{FocusPanel, ViewMode};

        if self.state.focus == FocusPanel::Files && !self.state.commit_files.is_empty() {
            if self.state.file_selected_index > 0 {
                self.state.file_selected_index -= 1;
                self.load_selected_file_diff();
            }
        } else if self.state.view_mode == ViewMode::Branches
            && self.state.branches_view_state.section == crate::state::BranchesSection::Stashes
        {
            let state = &mut self.state.branches_view_state;
            if let Some(stash) = state.stashes.get(state.stash_selected) {
                if !stash.files.is_empty() && state.stash_file_selected > 0 {
                    state.stash_file_selected -= 1;
                    self.load_stash_file_diff();
                }
            }
        }
    }

    fn handle_file_down(&mut self) {
        use crate::state::{FocusPanel, ViewMode};

        if self.state.focus == FocusPanel::Files && !self.state.commit_files.is_empty() {
            if self.state.file_selected_index + 1 < self.state.commit_files.len() {
                self.state.file_selected_index += 1;
                self.load_selected_file_diff();
            }
        } else if self.state.view_mode == ViewMode::Branches
            && self.state.branches_view_state.section == crate::state::BranchesSection::Stashes
        {
            let state = &mut self.state.branches_view_state;
            if let Some(stash) = state.stashes.get(state.stash_selected) {
                if !stash.files.is_empty() && state.stash_file_selected + 1 < stash.files.len() {
                    state.stash_file_selected += 1;
                    self.load_stash_file_diff();
                }
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
        {
            let is_amending = self.state.staging_state.is_amending;

            if is_amending {
                // Mode amendement
                crate::git::commit::amend_commit(
                    &self.state.repo.repo,
                    &self.state.staging_state.commit_message,
                )?;
                self.state
                    .set_flash_message("Commit amendé avec succès".into());
            } else {
                // Mode création de commit normal
                if self.state.staging_state.staged_files.is_empty() {
                    self.state
                        .set_flash_message("Aucun fichier staged pour le commit".into());
                    return Ok(());
                }

                crate::git::commit::create_commit(
                    &self.state.repo.repo,
                    &self.state.staging_state.commit_message,
                )?;
                self.state
                    .set_flash_message("Commit créé avec succès".into());
            }

            self.state.staging_state.commit_message.clear();
            self.state.staging_state.cursor_position = 0;
            self.state.staging_state.is_committing = false;
            self.state.staging_state.is_amending = false;
            self.state.mark_dirty(); // Marquer comme modifié
            self.refresh_staging()?;
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

        // Gérer la saisie dans la recherche
        if self.state.search_state.is_active {
            let byte_pos = self.char_to_byte_position(
                &self.state.search_state.query,
                self.state.search_state.cursor,
            );
            self.state.search_state.query.insert(byte_pos, c);
            self.state.search_state.cursor += 1;
            // Effectuer la recherche à chaque caractère
            self.perform_search();
            return;
        }

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

        // Gérer la suppression dans la recherche
        if self.state.search_state.is_active && self.state.search_state.cursor > 0 {
            self.state.search_state.cursor -= 1;
            let byte_pos = self.char_to_byte_position(
                &self.state.search_state.query,
                self.state.search_state.cursor,
            );
            let end_byte_pos = self.char_to_byte_position(
                &self.state.search_state.query,
                self.state.search_state.cursor + 1,
            );
            self.state.search_state.query.drain(byte_pos..end_byte_pos);
            // Effectuer la recherche après suppression
            self.perform_search();
            return;
        }

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

    /// Déclenche la confirmation pour la suppression d'une branche.
    fn handle_branch_delete_trigger(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        match self.state.view_mode {
            ViewMode::Graph => {
                // Dans la vue Graph, on supprime la branche du commit sélectionné
                if let Some(row) = self.state.graph.get(self.state.selected_index) {
                    // Trouver les branches pointant vers ce commit
                    let branches: Vec<String> = row
                        .node
                        .refs
                        .iter()
                        .filter(|r| !r.contains("->") && !r.starts_with("tag:"))
                        .map(|r| r.clone())
                        .collect();

                    if branches.is_empty() {
                        self.state
                            .set_flash_message("Aucune branche sur ce commit".into());
                    } else if branches.len() == 1 {
                        let name = branches[0].clone();
                        // Vérifier si c'est la branche courante
                        if let Some(current) = &self.state.current_branch {
                            if &name == current {
                                self.state.set_flash_message(
                                    "Impossible de supprimer la branche courante (HEAD)".into(),
                                );
                                return Ok(());
                            }
                        }
                        self.state.pending_confirmation = Some(ConfirmAction::BranchDelete(name));
                    } else {
                        // Plusieurs branches : afficher un message pour l'instant
                        self.state.set_flash_message(format!(
                            "Plusieurs branches sur ce commit: {}",
                            branches.join(", ")
                        ));
                    }
                }
            }
            ViewMode::Branches => {
                // Dans la vue Branches, on supprime la branche sélectionnée
                if self.state.branches_view_state.section == crate::state::BranchesSection::Branches
                {
                    if let Some(branch) = self
                        .state
                        .branches_view_state
                        .local_branches
                        .get(self.state.branches_view_state.branch_selected)
                    {
                        let name = branch.name.clone();
                        // Vérifier si c'est la branche courante
                        if let Some(current) = &self.state.current_branch {
                            if &name == current {
                                self.state.set_flash_message(
                                    "Impossible de supprimer la branche courante (HEAD)".into(),
                                );
                                return Ok(());
                            }
                        }
                        self.state.pending_confirmation = Some(ConfirmAction::BranchDelete(name));
                    }
                }
            }
            _ => {}
        }
        Ok(())
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
                    // Déclencher le dialogue de confirmation
                    self.state.pending_confirmation = Some(ConfirmAction::WorktreeRemove(name));
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
                // Déclencher le dialogue de confirmation
                self.state.pending_confirmation = Some(ConfirmAction::StashDrop(idx));
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

    fn handle_stash_selected_file(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            if let Some(file) = self
                .state
                .staging_state
                .unstaged_files
                .get(self.state.staging_state.unstaged_selected)
            {
                let file_path = file.path.clone();
                match crate::git::stash::stash_file(&self.state.repo_path, &file_path, None) {
                    Ok(_) => {
                        self.state
                            .set_flash_message(format!("Fichier '{}' stashé", file_path));
                        self.state.mark_dirty();
                        self.refresh_staging()?;
                    }
                    Err(e) => {
                        self.state.set_flash_message(format!("Erreur: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_stash_unstaged_files(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Staging {
            match crate::git::stash::stash_unstaged_files(&self.state.repo_path, None) {
                Ok(_) => {
                    self.state
                        .set_flash_message("Fichiers unstaged stashés".into());
                    self.state.mark_dirty();
                    self.refresh_staging()?;
                }
                Err(e) => {
                    self.state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
        Ok(())
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

    fn handle_branch_create(&mut self) -> Result<()> {
        use crate::state::{BranchesFocus, InputAction, ViewMode};

        // Disponible dans les vues Graph et Branches
        match self.state.view_mode {
            ViewMode::Graph => {
                // Dans la vue Graph, on passe en mode Branches avec input activé
                self.state.view_mode = ViewMode::Branches;
                self.state.branches_view_state.focus = BranchesFocus::Input;
                self.state.branches_view_state.input_action = Some(InputAction::CreateBranch);
                self.state.branches_view_state.input_text.clear();
                self.state.branches_view_state.input_cursor = 0;
                self.refresh_branches_view()?;
            }
            ViewMode::Branches => {
                // Dans la vue Branches, on active juste l'input
                self.state.branches_view_state.focus = BranchesFocus::Input;
                self.state.branches_view_state.input_action = Some(InputAction::CreateBranch);
                self.state.branches_view_state.input_text.clear();
                self.state.branches_view_state.input_cursor = 0;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_commit_prompt(&mut self) -> Result<()> {
        use crate::state::{StagingFocus, ViewMode};

        // Rediriger vers la vue Staging avec le focus sur le commit message
        self.state.view_mode = ViewMode::Staging;

        // Rafraîchir les données de staging si nécessaire
        let all_entries = self.state.repo.status().unwrap_or_default();
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

        // Activer directement le mode commit message
        self.state.staging_state.focus = StagingFocus::CommitMessage;
        self.state.staging_state.is_committing = true;
        self.state.staging_state.commit_message.clear();
        self.state.staging_state.cursor_position = 0;

        Ok(())
    }

    fn handle_stash_prompt(&mut self) -> Result<()> {
        use crate::state::{BranchesFocus, BranchesSection, InputAction, ViewMode};

        // Ouvrir un overlay pour saisir le message du stash
        self.state.view_mode = ViewMode::Branches;
        self.state.branches_view_state.section = BranchesSection::Stashes;
        self.state.branches_view_state.focus = BranchesFocus::Input;
        self.state.branches_view_state.input_action = Some(InputAction::SaveStash);
        self.state.branches_view_state.input_text.clear();
        self.state.branches_view_state.input_cursor = 0;
        self.refresh_branches_view()?;

        Ok(())
    }

    fn handle_merge_prompt(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        if self.state.view_mode == ViewMode::Branches {
            // Cas 1 : Vue Branches → merge direct de la branche focusée
            let branch = self
                .state
                .branches_view_state
                .local_branches
                .get(self.state.branches_view_state.branch_selected)
                .cloned();
            if let Some(branch) = branch {
                if branch.is_head {
                    self.state.set_flash_message(
                        "Impossible de merger la branche courante dans elle-même".into(),
                    );
                } else {
                    // Demander confirmation
                    let current_branch = self.state.current_branch.clone().unwrap_or_default();
                    self.state.pending_confirmation = Some(ConfirmAction::MergeBranch(
                        branch.name.clone(),
                        current_branch,
                    ));
                }
            }
        } else {
            // Cas 2 : Autres vues → ouvrir le sélecteur de branches
            self.open_merge_picker()?;
        }
        Ok(())
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

    fn load_stash_file_diff(&mut self) {
        let state = &mut self.state.branches_view_state;

        if let Some(stash) = state.stashes.get(state.stash_selected) {
            if let Some(file) = stash.files.get(state.stash_file_selected) {
                match self.state.repo.stash_file_diff(stash.oid, &file.path) {
                    Ok(diff_lines) => {
                        state.stash_file_diff = Some(diff_lines);
                    }
                    Err(_) => {
                        state.stash_file_diff = None;
                    }
                }
            } else {
                state.stash_file_diff = None;
            }
        } else {
            state.stash_file_diff = None;
        }
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
                let local_count = self.state.branches_view_state.local_branches.len();
                let remote_count = self.state.branches_view_state.remote_branches.len();
                let show_remote = self.state.branches_view_state.show_remote;

                let max = if show_remote && remote_count > 0 {
                    local_count + remote_count
                } else {
                    local_count
                };

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

                    // Charger le diff du fichier sélectionné
                    self.load_stash_file_diff();
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

        // Charger le diff du fichier de stash sélectionné
        self.load_stash_file_diff();

        Ok(())
    }

    /// Gère l'opération Git Push.
    fn handle_git_push(&mut self) -> Result<()> {
        // Vérifier si un remote est configuré
        match crate::git::remote::has_remote(&self.state.repo.repo) {
            Ok(true) => {
                // Lancer le push
                match crate::git::remote::push_current_branch(&self.state.repo.repo) {
                    Ok(msg) => {
                        self.state.set_flash_message(format!("{} ✓", msg));
                    }
                    Err(e) => {
                        self.state
                            .set_flash_message(format!("Erreur lors du push: {}", e));
                    }
                }
            }
            Ok(false) => {
                self.state
                    .set_flash_message("Aucun remote configuré".into());
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur: {}", e));
            }
        }
        Ok(())
    }

    /// Gère l'opération Git Pull.
    fn handle_git_pull(&mut self) -> Result<()> {
        use crate::git::conflict::MergeResult;
        use crate::state::{ConflictsState, ViewMode};

        // Vérifier si un remote est configuré
        match crate::git::remote::has_remote(&self.state.repo.repo) {
            Ok(true) => {
                // Lancer le pull
                match crate::git::remote::pull_current_branch_with_result(&self.state.repo.repo) {
                    Ok(MergeResult::UpToDate) => {
                        self.state.set_flash_message("Déjà à jour ✓".into());
                    }
                    Ok(MergeResult::FastForward) => {
                        self.state
                            .set_flash_message("Pull (fast-forward) réussi ✓".into());
                        self.state.mark_dirty();
                        self.refresh()?;
                    }
                    Ok(MergeResult::Success) => {
                        self.state.set_flash_message("Pull réussi ✓".into());
                        self.state.mark_dirty();
                        self.refresh()?;
                    }
                    Ok(MergeResult::Conflicts(files)) => {
                        let ours_name =
                            crate::git::conflict::get_current_branch_name(&self.state.repo.repo);
                        // Pour un pull, on utilise "origin" comme nom de branche theirs
                        // car la branche de tracking est souvent origin/<branche_courante>
                        let theirs_name = format!(
                            "origin/{}",
                            self.state.current_branch.clone().unwrap_or_else(|| "HEAD".to_string())
                        );
                        self.state.conflicts_state = Some(ConflictsState::new(
                            files,
                            "Pull depuis origin".into(),
                            ours_name,
                            theirs_name,
                        ));
                        self.state.view_mode = ViewMode::Conflicts;
                        self.state
                            .set_flash_message("Conflits détectés lors du pull".into());
                    }
                    Err(e) => {
                        self.state
                            .set_flash_message(format!("Erreur lors du pull: {}", e));
                    }
                }
            }
            Ok(false) => {
                self.state
                    .set_flash_message("Aucun remote configuré".into());
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur: {}", e));
            }
        }
        Ok(())
    }

    /// Gère l'opération Git Fetch.
    fn handle_git_fetch(&mut self) -> Result<()> {
        // Vérifier si un remote est configuré
        match crate::git::remote::has_remote(&self.state.repo.repo) {
            Ok(true) => {
                // Lancer le fetch
                match crate::git::remote::fetch_all(&self.state.repo.repo) {
                    Ok(_) => {
                        self.state.set_flash_message("Fetch réussi ✓".into());
                        self.state.mark_dirty();
                        self.refresh()?;
                    }
                    Err(e) => {
                        self.state
                            .set_flash_message(format!("Erreur lors du fetch: {}", e));
                    }
                }
            }
            Ok(false) => {
                self.state
                    .set_flash_message("Aucun remote configuré".into());
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur: {}", e));
            }
        }
        Ok(())
    }

    /// Ouvre le mode recherche.
    fn handle_open_search(&mut self) {
        self.state.search_state.is_active = true;
        self.state.search_state.query.clear();
        self.state.search_state.cursor = 0;
        self.state.search_state.results.clear();
        self.state.search_state.current_result = 0;
    }

    /// Ferme le mode recherche.
    fn handle_close_search(&mut self) {
        self.state.search_state.is_active = false;
        self.state.search_state.query.clear();
        self.state.search_state.results.clear();
    }

    /// Change le type de recherche (message → auteur → hash → message...).
    fn handle_change_search_type(&mut self) {
        use crate::git::search::SearchType;

        self.state.search_state.search_type = match self.state.search_state.search_type {
            SearchType::Message => SearchType::Author,
            SearchType::Author => SearchType::Hash,
            SearchType::Hash => SearchType::Message,
        };

        // Relancer la recherche avec le nouveau type
        self.perform_search();
    }

    /// Va au résultat de recherche suivant.
    fn handle_next_search_result(&mut self) {
        if !self.state.search_state.results.is_empty() {
            self.state.search_state.current_result = (self.state.search_state.current_result + 1)
                % self.state.search_state.results.len();

            // Mettre à jour la sélection du graphe
            let idx = self.state.search_state.results[self.state.search_state.current_result];
            self.state.selected_index = idx;
            self.state.graph_state.select(Some(idx * 2));
            self.update_commit_files();
        }
    }

    /// Va au résultat de recherche précédent.
    fn handle_prev_search_result(&mut self) {
        if !self.state.search_state.results.is_empty() {
            if self.state.search_state.current_result == 0 {
                self.state.search_state.current_result = self.state.search_state.results.len() - 1;
            } else {
                self.state.search_state.current_result -= 1;
            }

            // Mettre à jour la sélection du graphe
            let idx = self.state.search_state.results[self.state.search_state.current_result];
            self.state.selected_index = idx;
            self.state.graph_state.select(Some(idx * 2));
            self.update_commit_files();
        }
    }

    /// Effectue la recherche sur le graphe.
    fn perform_search(&mut self) {
        self.state.search_state.results = crate::git::search::filter_commits(
            &self.state.graph,
            &self.state.search_state.query,
            self.state.search_state.search_type.clone(),
        );
        self.state.search_state.current_result = 0;

        // Si on a des résultats, naviguer vers le premier
        if !self.state.search_state.results.is_empty() {
            let idx = self.state.search_state.results[0];
            self.state.selected_index = idx;
            self.state.graph_state.select(Some(idx * 2));
            self.update_commit_files();
        }
    }

    /// Déclenche le discard d'un fichier (demande confirmation).
    fn handle_discard_file(&mut self) -> Result<()> {
        use crate::state::StagingFocus;

        // Seulement dans la vue Staging et focus sur unstaged
        if !matches!(self.state.view_mode, crate::state::ViewMode::Staging) {
            return Ok(());
        }

        if !matches!(self.state.staging_state.focus, StagingFocus::Unstaged) {
            return Ok(());
        }

        // Récupérer le fichier sélectionné
        let selected = self.state.staging_state.unstaged_selected;
        if selected < self.state.staging_state.unstaged_files.len() {
            let file = &self.state.staging_state.unstaged_files[selected];
            let path = file.path.clone();

            // Demander confirmation
            self.state.pending_confirmation = Some(ConfirmAction::DiscardFile(path));
        }

        Ok(())
    }

    /// Déclenche le discard de tous les fichiers (demande confirmation).
    fn handle_discard_all(&mut self) -> Result<()> {
        // Seulement dans la vue Staging
        if !matches!(self.state.view_mode, crate::state::ViewMode::Staging) {
            return Ok(());
        }

        // Vérifier qu'il y a des fichiers unstaged
        if self.state.staging_state.unstaged_files.is_empty() {
            self.state
                .set_flash_message("Aucune modification à discard".into());
            return Ok(());
        }

        // Demander confirmation
        self.state.pending_confirmation = Some(ConfirmAction::DiscardAll);

        Ok(())
    }

    /// Exécute le discard d'un fichier.
    fn execute_discard_file(&mut self, path: &str) -> Result<()> {
        match crate::git::discard::discard_file(&self.state.repo.repo, path) {
            Ok(_) => {
                self.state
                    .set_flash_message(format!("Fichier '{}' restauré ✓", path));
                self.state.mark_dirty();
                self.refresh()?;
            }
            Err(e) => {
                self.state
                    .set_flash_message(format!("Erreur lors du discard: {}", e));
            }
        }
        Ok(())
    }

    /// Exécute le discard de tous les fichiers.
    fn execute_discard_all(&mut self) -> Result<()> {
        match crate::git::discard::discard_all(&self.state.repo.repo) {
            Ok(_) => {
                self.state
                    .set_flash_message("Toutes les modifications ont été restaurées ✓".into());
                self.state.mark_dirty();
                self.refresh()?;
            }
            Err(e) => {
                self.state
                    .set_flash_message(format!("Erreur lors du discard: {}", e));
            }
        }
        Ok(())
    }

    /// Gère la navigation dans la vue blame.
    fn handle_blame_navigation(&mut self, delta: i32) {
        if let Some(ref mut blame_state) = self.state.blame_state {
            if let Some(ref blame) = blame_state.blame {
                if blame.lines.is_empty() {
                    return;
                }

                let max_index = blame.lines.len() - 1;
                let new_index = if delta < 0 {
                    blame_state
                        .selected_line
                        .saturating_sub(delta.abs() as usize)
                } else {
                    (blame_state.selected_line + delta as usize).min(max_index)
                };

                blame_state.selected_line = new_index;

                // Ajuster le scroll si nécessaire
                // (La logique de scroll sera gérée par le widget de rendu)
            }
        }
    }

    /// Ouvre la vue blame pour le fichier sélectionné.
    fn handle_open_blame(&mut self) -> Result<()> {
        use crate::state::{BlameState, ViewMode};

        // Seulement depuis la vue Graph avec le focus sur Files
        if !matches!(self.state.view_mode, ViewMode::Graph) {
            return Ok(());
        }

        if !matches!(self.state.focus, crate::state::FocusPanel::Files) {
            return Ok(());
        }

        // Récupérer le fichier sélectionné
        if self.state.commit_files.is_empty() {
            self.state
                .set_flash_message("Aucun fichier sélectionné".into());
            return Ok(());
        }

        let selected_file = &self.state.commit_files[self.state.file_selected_index];
        let file_path = selected_file.path.clone();

        // Récupérer le commit sélectionné
        let commit_oid = if let Some(row) = self.state.graph.get(self.state.selected_index) {
            row.node.oid
        } else {
            self.state
                .set_flash_message("Aucun commit sélectionné".into());
            return Ok(());
        };

        // Créer l'état du blame
        let mut blame_state = BlameState::new(file_path.clone(), commit_oid);

        // Générer le blame
        match crate::git::blame::blame_file(&self.state.repo.repo, commit_oid, &file_path) {
            Ok(blame) => {
                blame_state.blame = Some(blame);
                self.state.blame_state = Some(blame_state);
                self.state.view_mode = ViewMode::Blame;
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur blame: {}", e));
            }
        }

        Ok(())
    }

    /// Ferme la vue blame.
    fn handle_close_blame(&mut self) {
        self.state.blame_state = None;
        self.state.view_mode = crate::state::ViewMode::Graph;
    }

    /// Navigue vers le commit du blame sélectionné.
    fn handle_jump_to_blame_commit(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        // Vérifier qu'on est bien en mode blame
        if !matches!(self.state.view_mode, ViewMode::Blame) {
            return Ok(());
        }

        // Récupérer le commit de la ligne sélectionnée
        if let Some(ref blame_state) = self.state.blame_state {
            if let Some(ref blame) = blame_state.blame {
                if let Some(line) = blame.lines.get(blame_state.selected_line) {
                    let target_oid = line.commit_oid;

                    // Fermer la vue blame
                    self.state.blame_state = None;
                    self.state.view_mode = ViewMode::Graph;

                    // Trouver l'index du commit dans le graphe
                    if let Some(index) = self
                        .state
                        .graph
                        .iter()
                        .position(|row| row.node.oid == target_oid)
                    {
                        self.state.selected_index = index;
                        self.state.graph_state.select(Some(index));
                        self.update_commit_files();
                    } else {
                        self.state.set_flash_message(format!(
                            "Commit {} non trouvé dans le graphe",
                            format!("{:.7}", target_oid)
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Cherry-pick le commit sélectionné.
    fn handle_cherry_pick(&mut self) -> Result<()> {
        use crate::state::ViewMode;

        // Seulement depuis la vue Graph
        if !matches!(self.state.view_mode, ViewMode::Graph) {
            return Ok(());
        }

        // Récupérer le commit sélectionné
        let commit_oid = if let Some(row) = self.state.graph.get(self.state.selected_index) {
            row.node.oid
        } else {
            self.state
                .set_flash_message("Aucun commit sélectionné".into());
            return Ok(());
        };

        // Demander confirmation via un dialogue
        self.state.pending_confirmation = Some(ConfirmAction::CherryPick(commit_oid));

        Ok(())
    }

    /// Exécute le cherry-pick.
    fn execute_cherry_pick(&mut self, commit_oid: git2::Oid) -> Result<()> {
        use crate::git::conflict::MergeResult;
        use crate::state::{ConflictsState, ViewMode};

        match crate::git::commit::cherry_pick_with_result(&self.state.repo.repo, commit_oid) {
            Ok(MergeResult::Success) => {
                self.state.set_flash_message(format!(
                    "Cherry-pick de {} réussi ✓",
                    format!("{:.7}", commit_oid)
                ));
                self.state.mark_dirty();
                self.refresh()?;
            }
            Ok(MergeResult::UpToDate) => {
                self.state.set_flash_message(format!(
                    "Cherry-pick de {} - déjà à jour",
                    format!("{:.7}", commit_oid)
                ));
            }
            Ok(MergeResult::Conflicts(files)) => {
                let ours_name =
                    crate::git::conflict::get_current_branch_name(&self.state.repo.repo);
                let theirs_name = format!("{:.7}", commit_oid);
                self.state.conflicts_state = Some(ConflictsState::new(
                    files,
                    format!("Cherry-pick de {}", format!("{:.7}", commit_oid)),
                    ours_name,
                    theirs_name,
                ));
                self.state.view_mode = ViewMode::Conflicts;
                self.state
                    .set_flash_message("Conflits détectés lors du cherry-pick".into());
            }
            Ok(MergeResult::FastForward) => {
                // Ne devrait pas arriver avec cherry-pick
                self.state.mark_dirty();
                self.refresh()?;
            }
            Err(e) => {
                self.state
                    .set_flash_message(format!("Erreur cherry-pick: {}", e));
            }
        }
        Ok(())
    }

    /// Amender le dernier commit.
    fn handle_amend_commit(&mut self) -> Result<()> {
        use crate::state::{StagingFocus, ViewMode};

        // Seulement depuis la vue Staging
        if !matches!(self.state.view_mode, ViewMode::Staging) {
            return Ok(());
        }

        // Récupérer le message du dernier commit
        let commit_message = {
            let head_commit = self.state.repo.repo.head()?.peel_to_commit()?;
            head_commit.message().unwrap_or("").to_string()
        };

        // Pré-remplir le message de commit
        self.state.staging_state.commit_message = commit_message;
        self.state.staging_state.cursor_position = self.state.staging_state.commit_message.len();
        self.state.staging_state.is_committing = true;
        self.state.staging_state.is_amending = true;
        self.state.staging_state.focus = StagingFocus::CommitMessage;

        self.state
            .set_flash_message("Mode amendement activé - éditez le message et validez".into());

        Ok(())
    }

    /// Ouvre le sélecteur de branches pour le merge.
    fn open_merge_picker(&mut self) -> Result<()> {
        use crate::state::MergePickerState;

        // Charger les branches si elles ne le sont pas déjà
        if self.state.branches_view_state.local_branches.is_empty() {
            let (local_branches, _) = crate::git::branch::list_all_branches(&self.state.repo.repo)?;
            self.state.branches_view_state.local_branches = local_branches;
        }

        // Récupérer la liste des branches locales (hors branche courante)
        let current_branch = self.state.current_branch.clone();
        let branches: Vec<String> = self
            .state
            .branches_view_state
            .local_branches
            .iter()
            .filter(|b| {
                if let Some(ref current) = current_branch {
                    b.name != *current
                } else {
                    true
                }
            })
            .map(|b| b.name.clone())
            .collect();

        if branches.is_empty() {
            self.state
                .set_flash_message("Aucune branche disponible pour le merge".into());
            return Ok(());
        }

        self.state.merge_picker = Some(MergePickerState {
            branches,
            selected: 0,
            is_active: true,
        });

        Ok(())
    }

    /// Navigation vers le haut dans le merge picker.
    fn handle_merge_picker_up(&mut self) {
        if let Some(ref mut picker) = self.state.merge_picker {
            if picker.selected > 0 {
                picker.selected -= 1;
            }
        }
    }

    /// Navigation vers le bas dans le merge picker.
    fn handle_merge_picker_down(&mut self) {
        if let Some(ref mut picker) = self.state.merge_picker {
            if !picker.branches.is_empty() && picker.selected + 1 < picker.branches.len() {
                picker.selected += 1;
            }
        }
    }

    /// Confirme la sélection dans le merge picker.
    fn handle_merge_picker_confirm(&mut self) -> Result<()> {
        let branch_to_merge = self
            .state
            .merge_picker
            .as_ref()
            .and_then(|picker| picker.branches.get(picker.selected))
            .cloned();

        if let Some(branch_name) = branch_to_merge {
            self.execute_merge(&branch_name)?;
        }

        self.state.merge_picker = None;
        Ok(())
    }

    /// Annule le merge picker.
    fn handle_merge_picker_cancel(&mut self) {
        self.state.merge_picker = None;
    }

    /// Exécute le merge d'une branche.
    fn execute_merge(&mut self, branch_name: &str) -> Result<()> {
        use crate::git::conflict::MergeResult;
        use crate::state::{ConflictsState, ViewMode};

        match crate::git::merge::merge_branch_with_result(&self.state.repo.repo, branch_name) {
            Ok(MergeResult::UpToDate) => {
                self.state
                    .set_flash_message(format!("Branche '{}' est déjà à jour", branch_name));
            }
            Ok(MergeResult::FastForward) => {
                self.state
                    .set_flash_message(format!("Fast-forward vers '{}'", branch_name));
                self.state.mark_dirty();
                self.refresh()?;
            }
            Ok(MergeResult::Success) => {
                self.state
                    .set_flash_message(format!("Branche '{}' mergée avec succès", branch_name));
                self.state.mark_dirty();
                self.refresh()?;
            }
            Ok(MergeResult::Conflicts(files)) => {
                let ours_name =
                    crate::git::conflict::get_current_branch_name(&self.state.repo.repo);
                let theirs_name = branch_name.to_string();
                self.state.conflicts_state = Some(ConflictsState::new(
                    files,
                    format!(
                        "Merge de '{}' dans '{}'",
                        branch_name,
                        self.state.current_branch.clone().unwrap_or_default()
                    ),
                    ours_name,
                    theirs_name,
                ));
                self.state.view_mode = ViewMode::Conflicts;
                self.state
                    .set_flash_message("Conflits détectés - Résolution requise".into());
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur: {}", e));
            }
        }
        Ok(())
    }

    // Handlers pour la vue de résolution de conflits

    fn handle_switch_to_conflicts(&mut self) {
        use crate::state::ViewMode;
        if self.state.conflicts_state.is_some() {
            self.state.view_mode = ViewMode::Conflicts;
        }
    }

    fn handle_conflict_enter_resolve(&mut self) -> Result<()> {
        use crate::git::conflict::{ConflictResolution, ConflictResolutionMode, ResolutionSide};
        use crate::state::ConflictPanelFocus;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            // Déterminer le côté selon le panneau actif
            let side = match conflicts_state.panel_focus {
                ConflictPanelFocus::OursPanel => ResolutionSide::Ours,
                ConflictPanelFocus::TheirsPanel => ResolutionSide::Theirs,
                _ => return Ok(()), // Pas de résolution depuis FileList ou Result
            };

            let mode = conflicts_state.resolution_mode;
            let resolution = match side {
                ResolutionSide::Ours => ConflictResolution::Ours,
                ResolutionSide::Theirs => ConflictResolution::Theirs,
            };

            if let Some(ref mut file) = conflicts_state
                .all_files
                .get_mut(conflicts_state.file_selected)
            {
                match mode {
                    ConflictResolutionMode::File => {
                        // En mode Fichier, résoudre toutes les sections d'un coup
                        for section in &mut file.conflicts {
                            section.resolution = Some(resolution);
                            // Mettre à jour aussi les line_level_resolution
                            if let Some(ref mut lr) = section.line_level_resolution {
                                match side {
                                    ResolutionSide::Ours => {
                                        lr.ours_lines_included.fill(true);
                                        lr.theirs_lines_included.fill(false);
                                    }
                                    ResolutionSide::Theirs => {
                                        lr.ours_lines_included.fill(false);
                                        lr.theirs_lines_included.fill(true);
                                    }
                                }
                                lr.touched = true;
                            }
                        }
                        file.is_resolved = true;
                    }
                    ConflictResolutionMode::Block => {
                        // En mode Block, résoudre seulement la section courante
                        if let Some(ref mut section) =
                            file.conflicts.get_mut(conflicts_state.section_selected)
                        {
                            section.resolution = Some(resolution);
                            file.is_resolved =
                                file.conflicts.iter().all(|s| s.resolution.is_some());
                        }
                    }
                    ConflictResolutionMode::Line => {
                        // En mode Ligne, toggle la ligne courante selon le panneau
                        if let Some(ref mut section) =
                            file.conflicts.get_mut(conflicts_state.section_selected)
                        {
                            let line_idx = conflicts_state.line_selected;
                            if let Some(ref mut lr) = section.line_level_resolution {
                                match side {
                                    ResolutionSide::Ours => {
                                        if line_idx < lr.ours_lines_included.len() {
                                            lr.ours_lines_included[line_idx] =
                                                !lr.ours_lines_included[line_idx];
                                            lr.touched = true;
                                        }
                                    }
                                    ResolutionSide::Theirs => {
                                        if line_idx < lr.theirs_lines_included.len() {
                                            lr.theirs_lines_included[line_idx] =
                                                !lr.theirs_lines_included[line_idx];
                                            lr.touched = true;
                                        }
                                    }
                                }
                                // Mettre à jour la résolution de section selon les lignes sélectionnées
                                let has_ours = lr.ours_lines_included.iter().any(|&b| b);
                                let has_theirs = lr.theirs_lines_included.iter().any(|&b| b);
                                section.resolution = match (has_ours, has_theirs) {
                                    (true, true) => Some(ConflictResolution::Both),
                                    (true, false) => Some(ConflictResolution::Ours),
                                    (false, true) => Some(ConflictResolution::Theirs),
                                    (false, false) => None,
                                };
                                file.is_resolved =
                                    file.conflicts.iter().all(|s| s.resolution.is_some());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_conflict_choose_both(&mut self) -> Result<()> {
        use crate::git::conflict::{ConflictResolution, ConflictResolutionMode};

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            let mode = conflicts_state.resolution_mode;
            let is_file_mode = mode == ConflictResolutionMode::File;
            let is_line_mode = mode == ConflictResolutionMode::Line;

            if let Some(ref mut file) = conflicts_state
                .all_files
                .get_mut(conflicts_state.file_selected)
            {
                if is_file_mode {
                    // En mode Fichier, résoudre toutes les sections d'un coup
                    for section in &mut file.conflicts {
                        section.resolution = Some(ConflictResolution::Both);
                        // Mettre à jour aussi les line_level_resolution
                        if let Some(ref mut lr) = section.line_level_resolution {
                            lr.ours_lines_included.fill(true);
                            lr.theirs_lines_included.fill(true);
                            lr.touched = true;
                        }
                    }
                    file.is_resolved = true;
                } else if is_line_mode {
                    // En mode Ligne, sélectionner toutes les lignes des deux côtés
                    if let Some(ref mut section) =
                        file.conflicts.get_mut(conflicts_state.section_selected)
                    {
                        if let Some(ref mut lr) = section.line_level_resolution {
                            lr.ours_lines_included.fill(true);
                            lr.theirs_lines_included.fill(true);
                            lr.touched = true;
                        }
                        section.resolution = Some(ConflictResolution::Both);
                        file.is_resolved = file.conflicts.iter().all(|s| s.resolution.is_some());
                    }
                } else {
                    // En mode Block, résoudre seulement la section courante
                    if let Some(ref mut section) =
                        file.conflicts.get_mut(conflicts_state.section_selected)
                    {
                        section.resolution = Some(ConflictResolution::Both);
                        file.is_resolved = file.conflicts.iter().all(|s| s.resolution.is_some());
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_conflict_file_choose_ours(&mut self) {
        use crate::git::conflict::ConflictResolution;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(ref mut file) = conflicts_state
                .all_files
                .get_mut(conflicts_state.file_selected)
            {
                // Résoudre toutes les sections en Ours
                for section in &mut file.conflicts {
                    section.resolution = Some(ConflictResolution::Ours);
                    // Réinitialiser les éventuelles résolutions par ligne
                    section.line_level_resolution = None;
                }
                file.is_resolved = true;

                // Avancer au fichier suivant non résolu
                self.advance_to_next_unresolved();
            }
        }
    }

    fn handle_conflict_file_choose_theirs(&mut self) {
        use crate::git::conflict::ConflictResolution;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(ref mut file) = conflicts_state
                .all_files
                .get_mut(conflicts_state.file_selected)
            {
                // Résoudre toutes les sections en Theirs
                for section in &mut file.conflicts {
                    section.resolution = Some(ConflictResolution::Theirs);
                    // Réinitialiser les éventuelles résolutions par ligne
                    section.line_level_resolution = None;
                }
                file.is_resolved = true;

                // Avancer au fichier suivant non résolu
                self.advance_to_next_unresolved();
            }
        }
    }

    fn advance_to_next_unresolved(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            let current = conflicts_state.file_selected;
            let total = conflicts_state.all_files.len();

            // Chercher le prochain fichier non résolu après le courant
            for i in 1..total {
                let idx = (current + i) % total;
                if let Some(file) = conflicts_state.all_files.get(idx) {
                    if !file.is_resolved {
                        conflicts_state.file_selected = idx;
                        conflicts_state.section_selected = 0;
                        conflicts_state.line_selected = 0;
                        conflicts_state.ours_scroll = 0;
                        conflicts_state.theirs_scroll = 0;
                        conflicts_state.result_scroll = 0;
                        return;
                    }
                }
            }

            // Si tous les fichiers suivants sont résolus, chercher depuis le début
            for i in 0..current {
                if let Some(file) = conflicts_state.all_files.get(i) {
                    if !file.is_resolved {
                        conflicts_state.file_selected = i;
                        conflicts_state.section_selected = 0;
                        conflicts_state.line_selected = 0;
                        conflicts_state.ours_scroll = 0;
                        conflicts_state.theirs_scroll = 0;
                        conflicts_state.result_scroll = 0;
                        return;
                    }
                }
            }
        }
    }

    fn handle_conflict_next_file(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.file_selected + 1 < conflicts_state.all_files.len() {
                conflicts_state.file_selected += 1;
                conflicts_state.section_selected = 0;
                conflicts_state.line_selected = 0;
                conflicts_state.ours_scroll = 0;
                conflicts_state.theirs_scroll = 0;
                conflicts_state.result_scroll = 0;
            }
        }
    }

    fn handle_conflict_prev_file(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.file_selected > 0 {
                conflicts_state.file_selected -= 1;
                conflicts_state.section_selected = 0;
                conflicts_state.line_selected = 0;
                conflicts_state.ours_scroll = 0;
                conflicts_state.theirs_scroll = 0;
                conflicts_state.result_scroll = 0;
            }
        }
    }

    /// Ajuste le scroll pour que la ligne sélectionnée reste visible.
    fn auto_scroll(scroll: &mut usize, selected_line: usize, panel_height: usize) {
        // Si la sélection est au-dessus de la zone visible
        if selected_line < *scroll {
            *scroll = selected_line;
        }
        // Si la sélection est en-dessous de la zone visible
        if selected_line >= *scroll + panel_height.saturating_sub(3) {
            *scroll = selected_line.saturating_sub(panel_height.saturating_sub(3));
        }
    }

    /// Calcule le numéro de ligne visuel pour une section donnée.
    fn calculate_section_scroll_line(
        file: &crate::git::conflict::MergeFile,
        section_idx: usize,
    ) -> usize {
        let mut line = 0usize;
        for (idx, section) in file.conflicts.iter().enumerate() {
            if idx == section_idx {
                return line;
            }
            // Compter les lignes de cette section
            // Titre de section (1 ligne) + context_before + max(ours, theirs) + context_after
            let section_lines = 1usize
                + section.context_before.len()
                + section.ours.len().max(section.theirs.len())
                + section.context_after.len()
                + 1; // Séparateur
            line += section_lines;
        }
        line
    }

    fn handle_conflict_next_section(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                if conflicts_state.section_selected + 1 < file.conflicts.len() {
                    conflicts_state.section_selected += 1;
                    conflicts_state.line_selected = 0;
                    // Calculer et synchroniser le scroll
                    let target_line = Self::calculate_section_scroll_line(
                        file,
                        conflicts_state.section_selected,
                    );
                    // Hauteur par défaut du panneau (sera ajustée par le rendu)
                    let panel_height = 20usize;
                    Self::auto_scroll(
                        &mut conflicts_state.ours_scroll,
                        target_line,
                        panel_height,
                    );
                    conflicts_state.theirs_scroll = conflicts_state.ours_scroll;
                }
            }
        }
    }

    fn handle_conflict_prev_section(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.section_selected > 0 {
                conflicts_state.section_selected -= 1;
                conflicts_state.line_selected = 0;
                // Calculer et synchroniser le scroll
                if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                    let target_line = Self::calculate_section_scroll_line(
                        file,
                        conflicts_state.section_selected,
                    );
                    let panel_height = 20usize;
                    Self::auto_scroll(
                        &mut conflicts_state.ours_scroll,
                        target_line,
                        panel_height,
                    );
                    conflicts_state.theirs_scroll = conflicts_state.ours_scroll;
                }
            }
        }
    }

    fn handle_conflict_resolve_file(&mut self) -> Result<()> {
        use crate::git::conflict::{
            resolve_file, resolve_special_file, ConflictFile, ConflictType,
        };

        // Extraire les informations nécessaires d'abord
        let file_info = {
            if let Some(ref conflicts_state) = self.state.conflicts_state {
                let file_selected = conflicts_state.file_selected;
                if let Some(file) = conflicts_state.all_files.get(file_selected) {
                    let is_special = matches!(
                        file.conflict_type,
                        Some(ConflictType::DeletedByUs)
                            | Some(ConflictType::DeletedByThem)
                            | Some(ConflictType::BothAdded)
                    );
                    // Vérifier si on est en mode édition avec buffer non vide
                    let use_edit_buffer =
                        conflicts_state.is_editing && !conflicts_state.edit_buffer.is_empty();

                    Some((
                        file.path.clone(),
                        file.conflicts.clone(),
                        file.is_resolved,
                        file.conflict_type,
                        file_selected,
                        is_special,
                        file.has_conflicts,
                        use_edit_buffer,
                        conflicts_state.edit_buffer.clone(), // Cloner le buffer si besoin
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some((
            file_path,
            conflicts,
            is_resolved,
            conflict_type,
            file_selected,
            is_special,
            has_conflicts,
            use_edit_buffer,
            edit_buffer,
        )) = file_info
        {
            if !has_conflicts {
                self.state
                    .set_flash_message("Ce fichier n'a pas de conflits".into());
                return Ok(());
            }

            // Mode édition: écrire directement le buffer
            if use_edit_buffer {
                let content = edit_buffer.join("\n");
                match std::fs::write(&file_path, content) {
                    Ok(()) => {
                        // Git add
                        let mut index = self.state.repo.repo.index()?;
                        index.add_path(std::path::Path::new(&file_path))?;
                        index.write()?;

                        // Mettre à jour le statut
                        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
                            if let Some(ref mut f) =
                                conflicts_state.all_files.get_mut(file_selected)
                            {
                                f.is_resolved = true;
                            }
                            conflicts_state.is_editing = false;
                        }
                        self.state.set_flash_message(format!(
                            "Fichier '{}' résolu (édition) ✓",
                            file_path
                        ));
                    }
                    Err(e) => {
                        self.state
                            .set_flash_message(format!("Erreur lors de l'écriture: {}", e));
                    }
                }
                return Ok(());
            }

            if is_special {
                // Pour les fichiers spéciaux, on a besoin d'une résolution
                let resolution = conflicts
                    .first()
                    .and_then(|s| s.resolution)
                    .unwrap_or(crate::git::conflict::ConflictResolution::Ours);

                // Créer un MergeFile temporaire pour resolve_special_file
                let merge_file = crate::git::conflict::MergeFile {
                    path: file_path.clone(),
                    has_conflicts: true,
                    conflicts: conflicts.clone(),
                    is_resolved,
                    conflict_type,
                };

                match resolve_special_file(&self.state.repo.repo, &merge_file, resolution) {
                    Ok(was_deleted) => {
                        // Mettre à jour le statut
                        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
                            if let Some(ref mut f) =
                                conflicts_state.all_files.get_mut(file_selected)
                            {
                                f.is_resolved = true;
                            }
                        }
                        if was_deleted {
                            self.state
                                .set_flash_message(format!("Fichier '{}' supprimé ✓", file_path));
                        } else {
                            self.state
                                .set_flash_message(format!("Fichier '{}' résolu ✓", file_path));
                        }
                    }
                    Err(e) => {
                        self.state
                            .set_flash_message(format!("Erreur lors de la résolution: {}", e));
                    }
                }
                return Ok(());
            }

            // Conflit classique (BothModified)
            if !conflicts.iter().all(|s| s.resolution.is_some()) {
                self.state
                    .set_flash_message("Toutes les sections ne sont pas résolues".into());
                return Ok(());
            }

            // Convertir MergeFile en ConflictFile pour resolve_file
            let conflict_file = ConflictFile {
                path: file_path.clone(),
                conflicts: conflicts.clone(),
                is_resolved,
                conflict_type: conflict_type.unwrap_or(ConflictType::BothModified),
            };

            match resolve_file(&self.state.repo.repo, &conflict_file) {
                Ok(()) => {
                    if let Some(ref mut conflicts_state) = self.state.conflicts_state {
                        if let Some(ref mut f) = conflicts_state.all_files.get_mut(file_selected) {
                            f.is_resolved = true;
                        }
                    }
                    self.state
                        .set_flash_message(format!("Fichier '{}' résolu ✓", file_path));
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur lors de la résolution: {}", e));
                }
            }
        }
        Ok(())
    }

    fn handle_conflict_finalize(&mut self) -> Result<()> {
        use crate::git::conflict::{count_unresolved_merge_files, finalize_merge};
        use crate::state::ViewMode;

        if let Some(ref conflicts_state) = self.state.conflicts_state {
            let unresolved = count_unresolved_merge_files(&conflicts_state.all_files);

            if unresolved > 0 {
                self.state.set_flash_message(format!(
                    "{} fichier(s) non résolu(s). Résolvez tous les conflits avant de finaliser.",
                    unresolved
                ));
                return Ok(());
            }

            // Créer le commit de merge
            let message = format!("Merge: {}", conflicts_state.operation_description);
            match finalize_merge(&self.state.repo.repo, &message) {
                Ok(()) => {
                    self.state.conflicts_state = None;
                    self.state.view_mode = ViewMode::Graph;
                    self.state
                        .set_flash_message("Merge finalisé avec succès ✓".into());
                    self.state.mark_dirty();
                    self.refresh()?;
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur lors de la finalisation: {}", e));
                }
            }
        }
        Ok(())
    }

    fn handle_conflict_abort(&mut self) -> Result<()> {
        use crate::git::conflict::abort_merge;
        use crate::state::ViewMode;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            match abort_merge(&self.state.repo.repo) {
                Ok(()) => {
                    let desc = conflicts_state.operation_description.clone();
                    self.state.conflicts_state = None;
                    self.state.view_mode = ViewMode::Graph;
                    self.state
                        .set_flash_message(format!("Merge annulé: {}", desc));
                    self.state.mark_dirty();
                    self.refresh()?;
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur lors de l'annulation: {}", e));
                }
            }
        }
        Ok(())
    }

    fn handle_conflict_set_mode_file(&mut self) -> Result<()> {
        use crate::git::conflict::ConflictResolutionMode;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.resolution_mode = ConflictResolutionMode::File;
            conflicts_state.line_selected = 0;
        }
        Ok(())
    }

    fn handle_conflict_set_mode_block(&mut self) -> Result<()> {
        use crate::git::conflict::ConflictResolutionMode;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.resolution_mode = ConflictResolutionMode::Block;
            conflicts_state.line_selected = 0;
        }
        Ok(())
    }

    fn handle_conflict_set_mode_line(&mut self) -> Result<()> {
        use crate::git::conflict::ConflictResolutionMode;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.resolution_mode = ConflictResolutionMode::Line;
            conflicts_state.line_selected = 0;
        }
        Ok(())
    }

    fn handle_conflict_toggle_line(&mut self) {
        use crate::git::conflict::ConflictResolutionMode;
        use crate::state::ConflictPanelFocus;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            // Vérifier qu'on est bien en mode Ligne
            if conflicts_state.resolution_mode != ConflictResolutionMode::Line {
                return;
            }

            let file_idx = conflicts_state.file_selected;
            let section_idx = conflicts_state.section_selected;
            let line_idx = conflicts_state.line_selected;
            let panel_focus = conflicts_state.panel_focus;

            if let Some(file) = conflicts_state.all_files.get_mut(file_idx) {
                if let Some(section) = file.conflicts.get_mut(section_idx) {
                    if let Some(ref mut lr) = section.line_level_resolution {
                        match panel_focus {
                            ConflictPanelFocus::OursPanel => {
                                if line_idx < lr.ours_lines_included.len() {
                                    lr.ours_lines_included[line_idx] = !lr.ours_lines_included[line_idx];
                                    lr.touched = true;
                                }
                            }
                            ConflictPanelFocus::TheirsPanel => {
                                if line_idx < lr.theirs_lines_included.len() {
                                    lr.theirs_lines_included[line_idx] = !lr.theirs_lines_included[line_idx];
                                    lr.touched = true;
                                }
                            }
                            _ => {}
                        }

                        // Mettre à jour le statut de résolution de la section
                        // Une section est considérée comme résolue si l'utilisateur a touché aux lignes
                        // et qu'au moins une ligne est sélectionnée
                        if lr.touched && lr.has_selection() {
                            section.resolution = Some(crate::git::conflict::ConflictResolution::Both);
                        }
                    }
                }
            }
        }
    }

    fn handle_conflict_line_down(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                if let Some(section) = file.conflicts.get(conflicts_state.section_selected) {
                    let max_lines = section.ours.len().max(section.theirs.len());
                    if conflicts_state.line_selected < max_lines {
                        conflicts_state.line_selected += 1;
                        // Mettre à jour le scroll pour suivre la ligne
                        let panel_height = 20usize;
                        let base_line = Self::calculate_section_scroll_line(
                            file,
                            conflicts_state.section_selected,
                        );
                        let target_line = base_line + 2 + conflicts_state.line_selected; // +2 pour le titre et context_before
                        Self::auto_scroll(
                            &mut conflicts_state.ours_scroll,
                            target_line,
                            panel_height,
                        );
                        conflicts_state.theirs_scroll = conflicts_state.ours_scroll;
                    }
                }
            }
        }
    }

    fn handle_conflict_line_up(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.line_selected > 0 {
                conflicts_state.line_selected -= 1;
                // Mettre à jour le scroll pour suivre la ligne
                if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                    let panel_height = 20usize;
                    let base_line = Self::calculate_section_scroll_line(
                        file,
                        conflicts_state.section_selected,
                    );
                    let target_line = base_line + 2 + conflicts_state.line_selected;
                    Self::auto_scroll(
                        &mut conflicts_state.ours_scroll,
                        target_line,
                        panel_height,
                    );
                    conflicts_state.theirs_scroll = conflicts_state.ours_scroll;
                }
            }
        }
    }

    fn handle_conflict_switch_panel_forward(&mut self) {
        use crate::state::ConflictPanelFocus;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.panel_focus = match conflicts_state.panel_focus {
                ConflictPanelFocus::FileList => ConflictPanelFocus::OursPanel,
                ConflictPanelFocus::OursPanel => ConflictPanelFocus::TheirsPanel,
                ConflictPanelFocus::TheirsPanel => ConflictPanelFocus::ResultPanel,
                ConflictPanelFocus::ResultPanel => ConflictPanelFocus::FileList,
            };
        }
    }

    fn handle_conflict_switch_panel_reverse(&mut self) {
        use crate::state::ConflictPanelFocus;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.panel_focus = match conflicts_state.panel_focus {
                ConflictPanelFocus::FileList => ConflictPanelFocus::ResultPanel,
                ConflictPanelFocus::ResultPanel => ConflictPanelFocus::TheirsPanel,
                ConflictPanelFocus::TheirsPanel => ConflictPanelFocus::OursPanel,
                ConflictPanelFocus::OursPanel => ConflictPanelFocus::FileList,
            };
        }
    }

    fn handle_conflict_result_scroll_down(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                // Calculer le nombre total de lignes dans le résultat
                let total_lines: usize = file
                    .conflicts
                    .iter()
                    .map(|s| {
                        s.context_before.len() + s.ours.len().max(s.theirs.len()) + s.context_after.len() + 2
                    })
                    .sum();
                let panel_height = 20usize;
                let max_scroll = total_lines.saturating_sub(panel_height);
                conflicts_state.result_scroll = (conflicts_state.result_scroll + 1).min(max_scroll);
            }
        }
    }

    fn handle_conflict_result_scroll_up(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.result_scroll = conflicts_state.result_scroll.saturating_sub(1);
        }
    }

    fn handle_conflict_validate_merge(&mut self) -> Result<()> {
        use crate::git::conflict::count_unresolved_merge_files;
        use crate::ui::confirm_dialog::ConfirmAction;

        if let Some(ref conflicts_state) = self.state.conflicts_state {
            let unresolved = count_unresolved_merge_files(&conflicts_state.all_files);

            if unresolved > 0 {
                self.state.set_flash_message(format!(
                    "{} fichier(s) non résolu(s). Résolvez tous les conflits avant de finaliser.",
                    unresolved
                ));
                return Ok(());
            }

            // Demander confirmation
            let desc = conflicts_state.operation_description.clone();
            self.state.pending_confirmation =
                Some(ConfirmAction::MergeBranch(desc, "merge".to_string()));
        }
        Ok(())
    }

    fn handle_conflict_start_editing(&mut self) {
        use crate::git::conflict::generate_resolved_content;

        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                // Générer le contenu résolu actuel
                let content = generate_resolved_content(file, conflicts_state.resolution_mode);
                conflicts_state.edit_buffer = content;
                conflicts_state.is_editing = true;
                conflicts_state.edit_cursor_line = 0;
                conflicts_state.edit_cursor_col = 0;
            }
        }
    }

    fn handle_conflict_stop_editing(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            conflicts_state.is_editing = false;
        }
    }

    fn handle_conflict_edit_insert_char(&mut self, c: char) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let line_idx = conflicts_state.edit_cursor_line;
                if line_idx < conflicts_state.edit_buffer.len() {
                    let col = conflicts_state.edit_cursor_col;
                    let line = &conflicts_state.edit_buffer[line_idx];
                    let mut new_line = line.chars().take(col).collect::<String>();
                    new_line.push(c);
                    new_line.push_str(&line.chars().skip(col).collect::<String>());
                    conflicts_state.edit_buffer[line_idx] = new_line;
                    conflicts_state.edit_cursor_col += 1;
                }
            }
        }
    }

    fn handle_conflict_edit_backspace(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let line_idx = conflicts_state.edit_cursor_line;
                if line_idx < conflicts_state.edit_buffer.len() {
                    let col = conflicts_state.edit_cursor_col;
                    if col > 0 {
                        // Supprimer le caractère avant le curseur
                        let line = &conflicts_state.edit_buffer[line_idx];
                        let mut new_line = line.chars().take(col - 1).collect::<String>();
                        new_line.push_str(&line.chars().skip(col).collect::<String>());
                        conflicts_state.edit_buffer[line_idx] = new_line;
                        conflicts_state.edit_cursor_col -= 1;
                    } else if line_idx > 0 {
                        // Fusionner avec la ligne précédente
                        let current_line = conflicts_state.edit_buffer.remove(line_idx);
                        let prev_line = &mut conflicts_state.edit_buffer[line_idx - 1];
                        let prev_len = prev_line.len();
                        prev_line.push_str(&current_line);
                        conflicts_state.edit_cursor_line -= 1;
                        conflicts_state.edit_cursor_col = prev_len;
                    }
                }
            }
        }
    }

    fn handle_conflict_edit_delete(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let line_idx = conflicts_state.edit_cursor_line;
                if line_idx < conflicts_state.edit_buffer.len() {
                    let col = conflicts_state.edit_cursor_col;
                    let line = &conflicts_state.edit_buffer[line_idx];
                    if col < line.len() {
                        // Supprimer le caractère sous le curseur
                        let mut new_line = line.chars().take(col).collect::<String>();
                        new_line.push_str(&line.chars().skip(col + 1).collect::<String>());
                        conflicts_state.edit_buffer[line_idx] = new_line;
                    } else if line_idx + 1 < conflicts_state.edit_buffer.len() {
                        // Fusionner avec la ligne suivante
                        let next_line = conflicts_state.edit_buffer.remove(line_idx + 1);
                        conflicts_state.edit_buffer[line_idx].push_str(&next_line);
                    }
                }
            }
        }
    }

    fn handle_conflict_edit_cursor_up(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing && conflicts_state.edit_cursor_line > 0 {
                conflicts_state.edit_cursor_line -= 1;
                // Ajuster la colonne si la ligne précédente est plus courte
                let line_len = conflicts_state.edit_buffer[conflicts_state.edit_cursor_line].len();
                if conflicts_state.edit_cursor_col > line_len {
                    conflicts_state.edit_cursor_col = line_len;
                }
            }
        }
    }

    fn handle_conflict_edit_cursor_down(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let max_line = conflicts_state.edit_buffer.len().saturating_sub(1);
                if conflicts_state.edit_cursor_line < max_line {
                    conflicts_state.edit_cursor_line += 1;
                    // Ajuster la colonne si la ligne suivante est plus courte
                    let line_len =
                        conflicts_state.edit_buffer[conflicts_state.edit_cursor_line].len();
                    if conflicts_state.edit_cursor_col > line_len {
                        conflicts_state.edit_cursor_col = line_len;
                    }
                }
            }
        }
    }

    fn handle_conflict_edit_cursor_left(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing && conflicts_state.edit_cursor_col > 0 {
                conflicts_state.edit_cursor_col -= 1;
            }
        }
    }

    fn handle_conflict_edit_cursor_right(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let line_idx = conflicts_state.edit_cursor_line;
                if line_idx < conflicts_state.edit_buffer.len() {
                    let line_len = conflicts_state.edit_buffer[line_idx].len();
                    if conflicts_state.edit_cursor_col < line_len {
                        conflicts_state.edit_cursor_col += 1;
                    }
                }
            }
        }
    }

    fn handle_conflict_edit_newline(&mut self) {
        if let Some(ref mut conflicts_state) = self.state.conflicts_state {
            if conflicts_state.is_editing {
                let line_idx = conflicts_state.edit_cursor_line;
                if line_idx < conflicts_state.edit_buffer.len() {
                    let col = conflicts_state.edit_cursor_col;
                    let line = conflicts_state.edit_buffer[line_idx].clone();
                    let new_line = line.chars().skip(col).collect::<String>();
                    conflicts_state.edit_buffer[line_idx] =
                        line.chars().take(col).collect::<String>();
                    conflicts_state.edit_buffer.insert(line_idx + 1, new_line);
                    conflicts_state.edit_cursor_line += 1;
                    conflicts_state.edit_cursor_col = 0;
                }
            }
        }
    }

    /// Copie le contenu du panneau actif dans le clipboard.
    fn handle_copy_panel_content(&mut self) -> Result<()> {
        use crate::state::{FocusPanel, StagingFocus, ViewMode};

        let mut text_to_copy = String::new();

        match self.state.view_mode {
            ViewMode::Graph => {
                // Graph view: copier hash + message du commit sélectionné
                if let Some(row) = self.state.graph.get(self.state.selected_index) {
                    let oid_str = row.node.oid.to_string();
                    let message = row.node.message.lines().next().unwrap_or("");
                    text_to_copy = format!("{} {}", oid_str, message);
                } else {
                    return Ok(());
                }

                // Ajouter le contenu du panneau Detail si focus est sur Files ou Detail
                match self.state.focus {
                    FocusPanel::Files => {
                        // Copier le chemin du fichier
                        if let Some(file) =
                            self.state.commit_files.get(self.state.file_selected_index)
                        {
                            text_to_copy = file.path.clone();
                            // Ajouter le diff si disponible
                            if let Some(ref diff) = self.state.selected_file_diff {
                                let diff_text = diff
                                    .lines
                                    .iter()
                                    .map(|line| line.content.trim_end_matches('\n').to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                text_to_copy = format!("{}\n\n{}", text_to_copy, diff_text);
                            }
                        }
                    }
                    FocusPanel::Detail => {
                        // Copier le diff
                        if let Some(ref diff) = self.state.selected_file_diff {
                            text_to_copy = diff
                                .lines
                                .iter()
                                .map(|line| line.content.trim_end_matches('\n').to_string())
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                    }
                    _ => {}
                }
            }
            ViewMode::Staging => {
                // Staging view: copier selon le focus
                match self.state.staging_state.focus {
                    StagingFocus::Unstaged => {
                        // Copier le chemin du fichier unstaged
                        text_to_copy = self
                            .state
                            .staging_state
                            .unstaged_files
                            .get(self.state.staging_state.unstaged_selected)
                            .map(|f| f.path.clone())
                            .unwrap_or_default();
                    }
                    StagingFocus::Staged => {
                        // Copier le chemin du fichier staged
                        text_to_copy = self
                            .state
                            .staging_state
                            .staged_files
                            .get(self.state.staging_state.staged_selected)
                            .map(|f| f.path.clone())
                            .unwrap_or_default();
                    }
                    StagingFocus::Diff => {
                        // Copier le contenu du diff
                        text_to_copy = self
                            .state
                            .staging_state
                            .current_diff
                            .as_ref()
                            .map(|diff| {
                                diff.lines
                                    .iter()
                                    .map(|line| line.content.trim_end_matches('\n').to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .unwrap_or_default();
                    }
                    StagingFocus::CommitMessage => {
                        // Copier le message de commit
                        text_to_copy = self.state.staging_state.commit_message.clone();
                    }
                }
            }
            ViewMode::Branches => {
                // Branches view: copier selon la section
                match self.state.branches_view_state.section {
                    crate::state::BranchesSection::Branches => {
                        text_to_copy = self
                            .state
                            .branches_view_state
                            .local_branches
                            .get(self.state.branches_view_state.branch_selected)
                            .map(|b| b.name.clone())
                            .unwrap_or_default();
                    }
                    crate::state::BranchesSection::Worktrees => {
                        text_to_copy = self
                            .state
                            .branches_view_state
                            .worktrees
                            .get(self.state.branches_view_state.worktree_selected)
                            .map(|w| format!("{} -> {}", w.name, w.path))
                            .unwrap_or_default();
                    }
                    crate::state::BranchesSection::Stashes => {
                        text_to_copy = self
                            .state
                            .branches_view_state
                            .stashes
                            .get(self.state.branches_view_state.stash_selected)
                            .map(|s| s.message.clone())
                            .unwrap_or_default();
                    }
                }
            }
            ViewMode::Blame => {
                // Blame view: copier le contenu de la ligne sélectionnée
                if let Some(ref blame_state) = self.state.blame_state {
                    if let Some(ref blame) = blame_state.blame {
                        text_to_copy = blame
                            .lines
                            .get(blame_state.selected_line)
                            .map(|l| l.content.clone())
                            .unwrap_or_default();
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            }
            ViewMode::Conflicts => {
                // Conflicts view: copier le chemin du fichier
                if let Some(ref conflicts_state) = self.state.conflicts_state {
                    text_to_copy = conflicts_state
                        .all_files
                        .get(conflicts_state.file_selected)
                        .map(|f| f.path.clone())
                        .unwrap_or_default();
                } else {
                    return Ok(());
                }
            }
            ViewMode::Help => {
                // Pas de contenu à copier dans l'aide
                return Ok(());
            }
        };

        // Copier dans le clipboard
        if !text_to_copy.is_empty() {
            match copy_to_clipboard(&text_to_copy) {
                Ok(_) => {
                    self.state
                        .set_flash_message("Copié dans le clipboard ✓".into());
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur clipboard: {}", e));
                }
            }
        }

        Ok(())
    }
}
