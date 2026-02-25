//! Dispatcher principal pour router les actions vers les handlers appropriés.
//!
//! Ce module remplace la logique monolithique de event.rs par un système
//! modulaire où chaque type d'action est géré par un handler spécialisé.

use crate::error::Result;
use crate::state::action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
use crate::state::{AppAction, AppState, BranchesFocus, FocusPanel, ViewMode};

use super::branch::BranchHandler;
use super::conflict::ConflictHandler;
use super::edit::EditHandler;
use super::filter::FilterHandler;
use super::git::GitHandler;
use super::navigation::NavigationHandler;
use super::search::SearchHandler;
use super::staging::StagingHandler;
use super::traits::{ActionHandler, HandlerContext};

/// Dispatcher qui route les actions vers les handlers appropriés.
pub struct ActionDispatcher {
    navigation: NavigationHandler,
    git: GitHandler,
    staging: StagingHandler,
    branch: BranchHandler,
    conflict: ConflictHandler,
    search: SearchHandler,
    edit: EditHandler,
    filter: FilterHandler,
}

impl ActionDispatcher {
    /// Crée un nouveau dispatcher avec tous les handlers initialisés.
    pub fn new() -> Self {
        Self {
            navigation: NavigationHandler,
            git: GitHandler,
            staging: StagingHandler,
            branch: BranchHandler,
            conflict: ConflictHandler,
            search: SearchHandler,
            edit: EditHandler,
            filter: FilterHandler,
        }
    }

    /// Dispatche une action vers le handler approprié.
    pub fn dispatch(&mut self, state: &mut AppState, action: AppAction) -> Result<()> {
        let mut ctx = HandlerContext { state };

        match action {
            // Actions imbriquées (nouvelle structure)
            AppAction::Navigation(nav) => self.navigation.handle(&mut ctx, nav),
            AppAction::Git(git) => self.git.handle(&mut ctx, git),
            AppAction::Staging(staging) => self.staging.handle(&mut ctx, staging),
            AppAction::Branch(branch) => self.branch.handle(&mut ctx, branch),
            AppAction::Conflict(conflict) => self.conflict.handle(&mut ctx, conflict),
            AppAction::Search(search) => self.search.handle(&mut ctx, search),
            AppAction::Edit(edit) => self.edit.handle(&mut ctx, edit),
            AppAction::Filter(filter) => self.filter.handle(&mut ctx, filter),

            // Actions simples
            AppAction::Quit => {
                ctx.state.should_quit = true;
                Ok(())
            }

            AppAction::Refresh => {
                ctx.state.dirty = true;
                Ok(())
            }

            AppAction::ToggleHelp => {
                if ctx.state.view_mode == ViewMode::Help {
                    // Retour à la vue précédente
                    ctx.state.view_mode = ctx
                        .state
                        .previous_view_mode
                        .take()
                        .unwrap_or(ViewMode::Graph);
                } else {
                    // Sauvegarder la vue courante et passer en mode Help
                    ctx.state.previous_view_mode = Some(ctx.state.view_mode);
                    ctx.state.view_mode = ViewMode::Help;
                }
                Ok(())
            }

            AppAction::SwitchBottomMode => {
                ctx.state.bottom_left_mode.toggle();
                Ok(())
            }

            AppAction::CloseBranchPanel => {
                ctx.state.show_branch_panel = false;
                Ok(())
            }

            AppAction::SwitchView(view_mode) => {
                ctx.state.view_mode = view_mode;
                ctx.state.dirty = true;
                Ok(())
            }

            AppAction::Select => {
                // En mode Graph avec focus sur Graph, Enter bascule vers le panneau fichiers (BottomLeft)
                // pour afficher les fichiers modifiés du commit sélectionné et leur diff.
                if ctx.state.view_mode == ViewMode::Graph && ctx.state.focus == FocusPanel::Graph {
                    ctx.state.focus = FocusPanel::BottomLeft;
                    // Réinitialiser la sélection de fichier pour commencer au début de la liste
                    ctx.state.file_selected_index = 0;
                    ctx.state.graph_view.file_selected_index = 0;
                    // S'assurer que les fichiers du commit actuel sont chargés
                    if let Some(row) = ctx.state.graph.get(ctx.state.selected_index) {
                        ctx.state.commit_files =
                            ctx.state.repo.commit_diff(row.node.oid).unwrap_or_default();
                    }
                    // Charger le diff du premier fichier
                    crate::handler::navigation::load_commit_file_diff(ctx.state);
                }
                Ok(())
            }

            AppAction::CopyToClipboard | AppAction::CopyPanelContent => {
                self.handle_copy_to_clipboard(&mut ctx)
            }

            // Merge picker actions
            AppAction::MergePickerUp => {
                if let Some(ref mut merge) = ctx.state.merge_picker {
                    let current = merge.selected();
                    if current > 0 {
                        merge.set_selected(current - 1);
                    }
                }
                Ok(())
            }

            AppAction::MergePickerDown => {
                if let Some(ref mut merge) = ctx.state.merge_picker {
                    let current = merge.selected();
                    let max = merge.branches.len();
                    if current + 1 < max {
                        merge.set_selected(current + 1);
                    }
                }
                Ok(())
            }

            AppAction::MergePickerConfirm => self.handle_merge_picker_confirm(&mut ctx),

            AppAction::MergePickerCancel => {
                ctx.state.merge_picker = None;
                Ok(())
            }

            // Confirmations
            AppAction::ConfirmAction => self.handle_confirm_action(&mut ctx),
            AppAction::CancelAction => {
                ctx.state.pending_confirmation = None;
                Ok(())
            }

            // ═══════════════════════════════════════════════════
            // Variantes legacy - redirection vers les nouveaux handlers
            // ═══════════════════════════════════════════════════

            // Navigation legacy
            AppAction::MoveUp => self.navigation.handle(&mut ctx, NavigationAction::MoveUp),
            AppAction::MoveDown => self.navigation.handle(&mut ctx, NavigationAction::MoveDown),
            AppAction::PageUp => self.navigation.handle(&mut ctx, NavigationAction::PageUp),
            AppAction::PageDown => self.navigation.handle(&mut ctx, NavigationAction::PageDown),
            AppAction::GoTop => self.navigation.handle(&mut ctx, NavigationAction::GoTop),
            AppAction::GoBottom => self.navigation.handle(&mut ctx, NavigationAction::GoBottom),
            AppAction::FileUp => self.navigation.handle(&mut ctx, NavigationAction::FileUp),
            AppAction::FileDown => self.navigation.handle(&mut ctx, NavigationAction::FileDown),
            AppAction::DiffScrollUp => self
                .navigation
                .handle(&mut ctx, NavigationAction::ScrollDiffUp),
            AppAction::DiffScrollDown => self
                .navigation
                .handle(&mut ctx, NavigationAction::ScrollDiffDown),

            // Git legacy
            AppAction::GitPush => self.git.handle(&mut ctx, GitAction::Push),
            AppAction::GitPull => self.git.handle(&mut ctx, GitAction::Pull),
            AppAction::GitFetch => self.git.handle(&mut ctx, GitAction::Fetch),
            AppAction::CherryPick => self.git.handle(&mut ctx, GitAction::CherryPick),
            AppAction::AmendCommit => self.git.handle(&mut ctx, GitAction::AmendCommit),
            AppAction::OpenBlame => self.git.handle(&mut ctx, GitAction::OpenBlame),
            AppAction::CloseBlame => self.git.handle(&mut ctx, GitAction::CloseBlame),
            AppAction::JumpToBlameCommit => self.git.handle(&mut ctx, GitAction::JumpToBlameCommit),
            AppAction::CommitPrompt => self.git.handle(&mut ctx, GitAction::CommitPrompt),
            AppAction::StashPrompt => self.git.handle(&mut ctx, GitAction::StashPrompt),
            AppAction::MergePrompt => self.git.handle(&mut ctx, GitAction::MergePrompt),
            AppAction::BranchList => self.git.handle(&mut ctx, GitAction::BranchList),

            // Staging legacy
            AppAction::StageFile => self.staging.handle(&mut ctx, StagingAction::StageFile),
            AppAction::UnstageFile => self.staging.handle(&mut ctx, StagingAction::UnstageFile),
            AppAction::StageAll => self.staging.handle(&mut ctx, StagingAction::StageAll),
            AppAction::UnstageAll => self.staging.handle(&mut ctx, StagingAction::UnstageAll),
            AppAction::SwitchStagingFocus => {
                self.staging.handle(&mut ctx, StagingAction::SwitchFocus)
            }
            AppAction::StartCommitMessage => self
                .staging
                .handle(&mut ctx, StagingAction::StartCommitMessage),
            AppAction::ConfirmCommit => self.staging.handle(&mut ctx, StagingAction::ConfirmCommit),
            AppAction::CancelCommitMessage => {
                self.staging.handle(&mut ctx, StagingAction::CancelCommit)
            }
            AppAction::DiscardFile => self.staging.handle(&mut ctx, StagingAction::DiscardFile),
            AppAction::DiscardAll => self.staging.handle(&mut ctx, StagingAction::DiscardAll),
            AppAction::StashSelectedFile => self
                .staging
                .handle(&mut ctx, StagingAction::StashSelectedFile),
            AppAction::StashUnstagedFiles => self
                .staging
                .handle(&mut ctx, StagingAction::StashUnstagedFiles),

            // Branch legacy
            AppAction::BranchCheckout => self.branch.handle(&mut ctx, BranchAction::Checkout),
            AppAction::BranchCreate => self.branch.handle(&mut ctx, BranchAction::Create),
            AppAction::BranchDelete => self.branch.handle(&mut ctx, BranchAction::Delete),
            AppAction::BranchRename => self.branch.handle(&mut ctx, BranchAction::Rename),
            AppAction::ToggleRemoteBranches => {
                self.branch.handle(&mut ctx, BranchAction::ToggleRemote)
            }
            AppAction::WorktreeCreate => self.branch.handle(&mut ctx, BranchAction::WorktreeCreate),
            AppAction::WorktreeRemove => self.branch.handle(&mut ctx, BranchAction::WorktreeRemove),
            AppAction::StashApply => self.branch.handle(&mut ctx, BranchAction::StashApply),
            AppAction::StashPop => self.branch.handle(&mut ctx, BranchAction::StashPop),
            AppAction::StashDrop => self.branch.handle(&mut ctx, BranchAction::StashDrop),
            AppAction::StashSave => self.branch.handle(&mut ctx, BranchAction::StashSave),
            AppAction::NextSection => self.branch.handle(&mut ctx, BranchAction::NextSection),
            AppAction::PrevSection => self.branch.handle(&mut ctx, BranchAction::PrevSection),
            AppAction::ConfirmInput => self.branch.handle(&mut ctx, BranchAction::ConfirmInput),
            AppAction::CancelInput => self.branch.handle(&mut ctx, BranchAction::CancelInput),

            // Search legacy
            AppAction::OpenSearch => self.search.handle(&mut ctx, SearchAction::Open),
            AppAction::CloseSearch => self.search.handle(&mut ctx, SearchAction::Close),
            AppAction::ChangeSearchType => self.search.handle(&mut ctx, SearchAction::ChangeType),
            AppAction::NextSearchResult => self.search.handle(&mut ctx, SearchAction::NextResult),
            AppAction::PrevSearchResult => {
                self.search.handle(&mut ctx, SearchAction::PreviousResult)
            }

            // Edit legacy
            AppAction::InsertChar(c) => {
                if ctx.state.view_mode == ViewMode::Branches
                    && ctx.state.branches_view_state.focus == BranchesFocus::Input
                {
                    // Modifier le texte de l'input branches
                    let pos = ctx.state.branches_view_state.input_cursor;
                    ctx.state.branches_view_state.input_text.insert(pos, c);
                    ctx.state.branches_view_state.input_cursor += 1;
                    Ok(())
                } else {
                    self.edit.handle(&mut ctx, EditAction::InsertChar(c))
                }
            }
            AppAction::DeleteChar => {
                if ctx.state.view_mode == ViewMode::Branches
                    && ctx.state.branches_view_state.focus == BranchesFocus::Input
                {
                    // Supprimer un caractère dans l'input branches
                    let pos = ctx.state.branches_view_state.input_cursor;
                    if pos > 0 {
                        ctx.state.branches_view_state.input_text.remove(pos - 1);
                        ctx.state.branches_view_state.input_cursor -= 1;
                    }
                    Ok(())
                } else {
                    self.edit.handle(&mut ctx, EditAction::DeleteCharBefore)
                }
            }
            AppAction::MoveCursorLeft => {
                if ctx.state.view_mode == ViewMode::Branches
                    && ctx.state.branches_view_state.focus == BranchesFocus::Input
                {
                    // Déplacer le curseur à gauche
                    if ctx.state.branches_view_state.input_cursor > 0 {
                        ctx.state.branches_view_state.input_cursor -= 1;
                    }
                    Ok(())
                } else {
                    self.edit.handle(&mut ctx, EditAction::CursorLeft)
                }
            }
            AppAction::MoveCursorRight => {
                if ctx.state.view_mode == ViewMode::Branches
                    && ctx.state.branches_view_state.focus == BranchesFocus::Input
                {
                    // Déplacer le curseur à droite
                    let len = ctx.state.branches_view_state.input_text.len();
                    if ctx.state.branches_view_state.input_cursor < len {
                        ctx.state.branches_view_state.input_cursor += 1;
                    }
                    Ok(())
                } else {
                    self.edit.handle(&mut ctx, EditAction::CursorRight)
                }
            }

            // View mode legacy
            AppAction::SwitchToGraph => {
                ctx.state.view_mode = ViewMode::Graph;
                ctx.state.dirty = true;
                Ok(())
            }
            AppAction::SwitchToStaging => {
                ctx.state.view_mode = ViewMode::Staging;
                ctx.state.dirty = true;
                // Charger le diff du premier fichier sélectionné
                crate::handler::staging::load_staging_diff(ctx.state);
                Ok(())
            }
            AppAction::SwitchToBranches => {
                ctx.state.view_mode = ViewMode::Branches;
                ctx.state.dirty = true;
                Ok(())
            }
            AppAction::SwitchToConflicts => {
                ctx.state.view_mode = ViewMode::Conflicts;
                ctx.state.dirty = true;
                Ok(())
            }

            // Conflict legacy
            AppAction::ConflictPrevFile => {
                self.conflict.handle(&mut ctx, ConflictAction::PreviousFile)
            }
            AppAction::ConflictNextFile => self.conflict.handle(&mut ctx, ConflictAction::NextFile),
            AppAction::ConflictPrevSection => self
                .conflict
                .handle(&mut ctx, ConflictAction::PreviousSection),
            AppAction::ConflictNextSection => {
                self.conflict.handle(&mut ctx, ConflictAction::NextSection)
            }
            AppAction::ConflictSwitchPanelForward => {
                self.conflict.handle(&mut ctx, ConflictAction::SwitchPanel)
            }
            AppAction::ConflictSwitchPanelReverse => {
                self.conflict.handle(&mut ctx, ConflictAction::SwitchPanel)
            }
            AppAction::ConflictFileChooseOurs => self
                .conflict
                .handle(&mut ctx, ConflictAction::AcceptOursFile),
            AppAction::ConflictFileChooseTheirs => self
                .conflict
                .handle(&mut ctx, ConflictAction::AcceptTheirsFile),
            AppAction::ConflictChooseBoth => {
                self.conflict.handle(&mut ctx, ConflictAction::AcceptBoth)
            }
            AppAction::ConflictFinalize => self
                .conflict
                .handle(&mut ctx, ConflictAction::FinalizeMerge),
            AppAction::ConflictValidateMerge => self
                .conflict
                .handle(&mut ctx, ConflictAction::FinalizeMerge),
            AppAction::ConflictAbort => self.conflict.handle(&mut ctx, ConflictAction::AbortMerge),
            AppAction::ConflictLeaveView => {
                self.conflict.handle(&mut ctx, ConflictAction::LeaveView)
            }
            AppAction::ConflictEnterResolve => {
                self.conflict.handle(&mut ctx, ConflictAction::EnterResolve)
            }
            AppAction::ConflictSetModeFile => {
                self.conflict.handle(&mut ctx, ConflictAction::SetModeFile)
            }
            AppAction::ConflictSetModeBlock => {
                self.conflict.handle(&mut ctx, ConflictAction::SetModeBlock)
            }
            AppAction::ConflictSetModeLine => {
                self.conflict.handle(&mut ctx, ConflictAction::SetModeLine)
            }
            AppAction::ConflictToggleLine => {
                self.conflict.handle(&mut ctx, ConflictAction::ToggleLine)
            }
            AppAction::ConflictLineUp => self.conflict.handle(&mut ctx, ConflictAction::LineUp),
            AppAction::ConflictLineDown => self.conflict.handle(&mut ctx, ConflictAction::LineDown),
            AppAction::ConflictResultScrollUp => self
                .conflict
                .handle(&mut ctx, ConflictAction::ResultScrollUp),
            AppAction::ConflictResultScrollDown => self
                .conflict
                .handle(&mut ctx, ConflictAction::ResultScrollDown),
            AppAction::ConflictStartEditing => {
                self.conflict.handle(&mut ctx, ConflictAction::StartEditing)
            }
            AppAction::ConflictStopEditing => {
                self.conflict.handle(&mut ctx, ConflictAction::StopEditing)
            }
            AppAction::ConflictConfirmEdit => {
                self.conflict.handle(&mut ctx, ConflictAction::ConfirmEdit)
            }
            AppAction::ConflictEditInsertChar(c) => self
                .conflict
                .handle(&mut ctx, ConflictAction::EditInsertChar(c)),
            AppAction::ConflictEditBackspace => self
                .conflict
                .handle(&mut ctx, ConflictAction::EditBackspace),
            AppAction::ConflictEditDelete => {
                self.conflict.handle(&mut ctx, ConflictAction::EditDelete)
            }
            AppAction::ConflictEditCursorUp => {
                self.conflict.handle(&mut ctx, ConflictAction::EditCursorUp)
            }
            AppAction::ConflictEditCursorDown => self
                .conflict
                .handle(&mut ctx, ConflictAction::EditCursorDown),
            AppAction::ConflictEditCursorLeft => self
                .conflict
                .handle(&mut ctx, ConflictAction::EditCursorLeft),
            AppAction::ConflictEditCursorRight => self
                .conflict
                .handle(&mut ctx, ConflictAction::EditCursorRight),
            AppAction::ConflictEditNewline => {
                self.conflict.handle(&mut ctx, ConflictAction::EditNewline)
            }
            AppAction::ConflictResolveFile => {
                self.conflict.handle(&mut ctx, ConflictAction::MarkResolved)
            }

            // Filter legacy
            AppAction::OpenFilter => self.filter.handle(&mut ctx, FilterAction::Open),
            AppAction::CloseFilter => self.filter.handle(&mut ctx, FilterAction::Close),
            AppAction::FilterNextField => self.filter.handle(&mut ctx, FilterAction::NextField),
            AppAction::FilterPrevField => self.filter.handle(&mut ctx, FilterAction::PreviousField),
            AppAction::ApplyFilter => self.filter.handle(&mut ctx, FilterAction::Apply),
            AppAction::ClearFilter => self.filter.handle(&mut ctx, FilterAction::Clear),
            AppAction::FilterInsertChar(c) => {
                self.filter.handle(&mut ctx, FilterAction::InsertChar(c))
            }
            AppAction::FilterDeleteChar => self.filter.handle(&mut ctx, FilterAction::DeleteChar),

            // Toggle diff view mode
            AppAction::ToggleDiffViewMode => {
                ctx.state.diff_view_mode.toggle();
                // Aussi toggle le mode dans la vue staging si on y est.
                ctx.state.staging_state.diff_view_mode.toggle();
                Ok(())
            }

            // Aucune action
            AppAction::None => Ok(()),
        }
    }

    /// Gère la copie dans le presse-papier.
    fn handle_copy_to_clipboard(&self, ctx: &mut HandlerContext) -> Result<()> {
        use crate::state::{BranchesSection, FocusPanel, StagingFocus};

        let mut text_to_copy = String::new();

        match ctx.state.view_mode {
            ViewMode::Graph => {
                // Graph view: copier hash + message du commit sélectionné
                if let Some(row) = ctx.state.graph.get(ctx.state.selected_index) {
                    let oid_str = row.node.oid.to_string();
                    let message = row.node.message.lines().next().unwrap_or("");
                    text_to_copy = format!("{} {}", oid_str, message);
                } else {
                    return Ok(());
                }

                // Ajouter le contenu du panneau BottomRight si focus est sur BottomLeft ou BottomRight
                match ctx.state.focus {
                    FocusPanel::BottomLeft => {
                        if let Some(file) =
                            ctx.state.commit_files.get(ctx.state.file_selected_index)
                        {
                            text_to_copy = file.path.clone();
                            if let Some(ref diff) = ctx.state.selected_file_diff {
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
                    FocusPanel::BottomRight => {
                        if let Some(ref diff) = ctx.state.selected_file_diff {
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
            ViewMode::Staging => match ctx.state.staging_state.focus {
                StagingFocus::Unstaged => {
                    text_to_copy = ctx
                        .state
                        .staging_state
                        .unstaged_files()
                        .get(ctx.state.staging_state.unstaged_selected())
                        .map(|f| f.path.clone())
                        .unwrap_or_default();
                }
                StagingFocus::Staged => {
                    text_to_copy = ctx
                        .state
                        .staging_state
                        .staged_files()
                        .get(ctx.state.staging_state.staged_selected())
                        .map(|f| f.path.clone())
                        .unwrap_or_default();
                }
                StagingFocus::Diff => {
                    text_to_copy = ctx
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
                    text_to_copy = ctx.state.staging_state.commit_message.clone();
                }
            },
            ViewMode::Branches => match ctx.state.branches_view_state.section {
                BranchesSection::Branches => {
                    text_to_copy = ctx
                        .state
                        .branches_view_state
                        .selected_branch()
                        .map(|b| b.name.clone())
                        .unwrap_or_default();
                }
                BranchesSection::Worktrees => {
                    text_to_copy = ctx
                        .state
                        .branches_view_state
                        .worktrees
                        .selected_item()
                        .map(|w| format!("{}: {}", w.name, w.path))
                        .unwrap_or_default();
                }
                BranchesSection::Stashes => {
                    text_to_copy = ctx
                        .state
                        .branches_view_state
                        .stashes
                        .selected_item()
                        .map(|s| {
                            format!(
                                "{}: {}",
                                s.oid.to_string().get(0..7).unwrap_or(""),
                                s.message
                            )
                        })
                        .unwrap_or_default();
                }
            },
            ViewMode::Conflicts => {
                if let Some(ref conflicts_state) = ctx.state.conflicts_state {
                    if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected)
                    {
                        text_to_copy = file.path.clone();
                    }
                }
            }
            ViewMode::Blame => {
                if let Some(ref blame_state) = ctx.state.blame_state {
                    if let Some(ref blame) = blame_state.blame {
                        text_to_copy = blame
                            .lines
                            .iter()
                            .map(|l| l.content.clone())
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
            }
            ViewMode::Help => {
                // Pas de contenu à copier en mode aide
            }
        }

        // Copier dans le clipboard
        if !text_to_copy.is_empty() {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
            clipboard
                .set_text(&text_to_copy)
                .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
            ctx.state
                .set_flash_message("Copié dans le presse-papier ✓".to_string());
        }

        Ok(())
    }

    /// Gère la confirmation du merge picker.
    fn handle_merge_picker_confirm(&self, ctx: &mut HandlerContext) -> Result<()> {
        use crate::git::conflict::MergeResult;

        let branch_to_merge = ctx
            .state
            .merge_picker
            .as_ref()
            .and_then(|picker| picker.branches.selected_item())
            .cloned();

        if let Some(branch_name) = branch_to_merge {
            match crate::git::merge::merge_branch_with_result(&ctx.state.repo.repo, &branch_name) {
                Ok(MergeResult::UpToDate) => {
                    ctx.state
                        .set_flash_message(format!("Branche '{}' est déjà à jour", branch_name));
                }
                Ok(MergeResult::FastForward) => {
                    ctx.state
                        .set_flash_message(format!("Fast-forward vers '{}'", branch_name));
                    ctx.state.mark_dirty();
                }
                Ok(MergeResult::Success) => {
                    ctx.state
                        .set_flash_message(format!("Branche '{}' mergée avec succès", branch_name));
                    ctx.state.mark_dirty();
                }
                Ok(MergeResult::Conflicts(conflicts)) => {
                    ctx.state.set_flash_message(format!(
                        "Conflits lors du merge avec '{}' ({} fichiers)",
                        branch_name,
                        conflicts.len()
                    ));
                    // Activer la vue conflits
                    let current = ctx
                        .state
                        .current_branch
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string());
                    ctx.state.conflicts_state = Some(crate::state::ConflictsState::new(
                        conflicts,
                        format!("merge {}", branch_name),
                        current,
                        branch_name,
                    ));
                    ctx.state.view_mode = ViewMode::Conflicts;
                }
                Err(e) => {
                    ctx.state.set_flash_message(format!("Erreur merge: {}", e));
                }
            }
        }

        ctx.state.merge_picker = None;
        Ok(())
    }

    /// Gère la confirmation d'une action destructive.
    fn handle_confirm_action(&self, ctx: &mut HandlerContext) -> Result<()> {
        use crate::ui::confirm_dialog::ConfirmAction;

        if let Some(confirm_action) = ctx.state.pending_confirmation.clone() {
            match confirm_action {
                ConfirmAction::DiscardAll => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::discard::discard_all(&ctx.state.repo.repo) {
                        ctx.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        ctx.state
                            .set_flash_message("Modifications ignorées ✓".to_string());
                    }
                    ctx.state.mark_dirty();
                }
                ConfirmAction::DiscardFile(path) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::discard::discard_file(&ctx.state.repo.repo, &path) {
                        ctx.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        ctx.state.set_flash_message(format!("{} ignoré ✓", path));
                    }
                    ctx.state.mark_dirty();
                }
                ConfirmAction::BranchDelete(name) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::branch::delete_branch(&ctx.state.repo.repo, &name) {
                        ctx.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        ctx.state
                            .set_flash_message(format!("Branche {} supprimée ✓", name));
                    }
                    ctx.state.mark_dirty();
                }
                ConfirmAction::AbortMerge => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::conflict::abort_merge(&ctx.state.repo.repo) {
                        ctx.state.set_flash_message(format!("Erreur: {}", e));
                    } else {
                        ctx.state.set_flash_message("Merge annulé ✓".to_string());
                        ctx.state.conflicts_state = None;
                    }
                    ctx.state.mark_dirty();
                }
                _ => {
                    ctx.state.pending_confirmation = None;
                }
            }
        }
        Ok(())
    }
}

impl Default for ActionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
