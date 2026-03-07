//! Dispatcher principal pour router les actions vers les handlers appropriés.
//!
//! Ce module remplace la logique monolithique de event.rs par un système
//! modulaire où chaque type d'action est géré par un handler spécialisé.

use crate::error::Result;
use crate::state::{AppAction, AppState, FocusPanel, ViewMode};

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
                    ctx.state.graph_view.file_selected_index = 0;
                    // Rafraîchir les fichiers du commit actuel
                    ctx.state.refresh_commit_files();
                    // Charger le diff du premier fichier
                    crate::handler::navigation::load_commit_file_diff(ctx.state);
                } else if ctx.state.view_mode == ViewMode::Graph
                    && ctx.state.focus == FocusPanel::BottomLeft
                {
                    // Depuis la liste de fichiers, Espace ouvre le panneau diff sans plein écran.
                    ctx.state.focus = FocusPanel::BottomRight;
                    ctx.state.graph_view.diff_fullscreen = false;
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

            // Reset picker actions
            AppAction::ResetPickerSelectSoft => {
                if let Some(ref mut reset) = ctx.state.reset_picker {
                    reset.selected_index = 0;
                }
                Ok(())
            }

            AppAction::ResetPickerSelectHard => {
                if let Some(ref mut reset) = ctx.state.reset_picker {
                    reset.selected_index = 1;
                }
                Ok(())
            }

            AppAction::ResetPickerConfirm => {
                if let Some(ref reset) = ctx.state.reset_picker {
                    let oid = reset.target_oid;
                    if reset.is_soft_selected() {
                        ctx.state.pending_confirmation =
                            Some(crate::ui::confirm_dialog::ConfirmAction::ResetSoft(oid));
                    } else {
                        ctx.state.pending_confirmation =
                            Some(crate::ui::confirm_dialog::ConfirmAction::ResetHard(oid));
                    }
                    ctx.state.reset_picker = None;
                }
                Ok(())
            }

            AppAction::ResetPickerCancel => {
                ctx.state.reset_picker = None;
                Ok(())
            }

            // Confirmations
            AppAction::ConfirmAction => self.handle_confirm_action(&mut ctx),
            AppAction::CancelAction => {
                ctx.state.pending_confirmation = None;
                Ok(())
            }

            // Toggle diff view mode
            AppAction::ToggleDiffViewMode => {
                ctx.state.graph_view.toggle_diff_view_mode();
                // Aussi toggle le mode dans la vue staging si on y est.
                ctx.state.staging_state.diff_view_mode.toggle();
                Ok(())
            }

            // Toggle diff fullscreen mode
            AppAction::ToggleDiffFullscreen => {
                let is_fullscreen = ctx.state.graph_view.diff_fullscreen;
                ctx.state.graph_view.diff_fullscreen = !is_fullscreen;
                if ctx.state.graph_view.diff_fullscreen {
                    ctx.state.focus = FocusPanel::BottomRight;
                } else if ctx.state.view_mode == ViewMode::Graph {
                    ctx.state.focus = FocusPanel::BottomLeft;
                    // Réinitialiser le scroll horizontal quand on sort du plein écran
                    ctx.state.graph_view.diff_horizontal_offset = 0;
                }
                Ok(())
            }

            // Charger plus d'historique (pagination)
            AppAction::LoadMoreHistory => self.handle_load_more_history(&mut ctx),

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
                if let Some(commit) = ctx.state.selected_commit() {
                    let oid_str = commit.oid.to_string();
                    let message = commit.message.lines().next().unwrap_or("");
                    text_to_copy = format!("{} {}", oid_str, message);
                } else {
                    return Ok(());
                }

                // Ajouter le contenu du panneau BottomRight si focus est sur BottomLeft ou BottomRight
                match ctx.state.focus {
                    FocusPanel::BottomLeft => {
                        if let Some(file) = ctx
                            .state
                            .graph_view
                            .commit_files
                            .get(ctx.state.graph_view.file_selected_index)
                        {
                            text_to_copy = file.path.clone();
                            if let Some(ref diff) = ctx.state.graph_view.selected_file_diff {
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
                        if let Some(ref diff) = ctx.state.graph_view.selected_file_diff {
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

    /// Gère le chargement progressif de l'historique.
    fn handle_load_more_history(&self, ctx: &mut HandlerContext) -> Result<()> {
        use crate::state::{COMMIT_BATCH_SIZE, MAX_TOTAL_COMMITS};

        // Vérifier si on peut charger plus
        if !ctx.state.graph_view.can_load_more {
            ctx.state
                .set_flash_message("Plus d'historique disponible".to_string());
            return Ok(());
        }

        // Vérifier si un chargement est déjà en cours
        if ctx.state.graph_view.is_loading_more {
            return Ok(());
        }

        // Marquer le début du chargement
        ctx.state.graph_view.start_loading_more();

        // Calculer combien de commits charger
        let current_count = ctx.state.graph_view.loaded_count;
        let target_count = (current_count + COMMIT_BATCH_SIZE).min(MAX_TOTAL_COMMITS);

        if target_count <= current_count {
            ctx.state.graph_view.finish_loading_more();
            ctx.state
                .set_flash_message("Limite d'historique atteinte".to_string());
            return Ok(());
        }

        // Charger les commits supplémentaires
        let additional_count = target_count - current_count;

        // Si c'est le premier chargement (current_count == 0), on charge INITIAL_COMMIT_COUNT
        // Sinon, on charge à partir de current_count
        let skip = if current_count == 0 { 0 } else { current_count };

        match ctx.state.repo.build_graph_offset(skip, additional_count) {
            Ok(additional_rows) => {
                if additional_rows.is_empty() {
                    // Plus de commits à charger
                    ctx.state.graph_view.can_load_more = false;
                    ctx.state
                        .set_flash_message("Fin de l'historique atteinte".to_string());
                } else {
                    // Ajouter les nouveaux commits au graphe existant
                    ctx.state.graph_view.append_commits(additional_rows);

                    // Mettre à jour l'état de pagination
                    let new_count = ctx.state.graph_view.loaded_count;
                    let total = ctx.state.repo.estimate_total_commits();
                    ctx.state
                        .graph_view
                        .update_pagination_state(new_count, total);

                    // Message de confirmation
                    let msg = if let Some(total) = total {
                        format!("{} / {} commits chargés", new_count, total)
                    } else {
                        format!("{} commits chargés", new_count)
                    };
                    ctx.state.set_flash_message(msg);
                }
            }
            Err(e) => {
                ctx.state
                    .set_flash_message(format!("Erreur chargement: {}", e));
            }
        }

        // Marquer la fin du chargement
        ctx.state.graph_view.finish_loading_more();

        Ok(())
    }

    /// Gère la confirmation d'une action destructive.
    fn handle_confirm_action(&self, ctx: &mut HandlerContext) -> Result<()> {
        use crate::git::conflict::MergeResult;
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
                        ctx.state.is_merging = false;
                    }
                    ctx.state.mark_dirty();
                }
                ConfirmAction::ResetSoft(oid) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::commit::reset_to_commit(
                        &ctx.state.repo.repo,
                        oid,
                        git2::ResetType::Soft,
                    ) {
                        ctx.state
                            .set_flash_message(format!("Erreur reset soft: {}", e));
                    } else {
                        ctx.state.set_flash_message(format!(
                            "Reset soft vers {} effectué ✓",
                            format!("{:.7}", oid)
                        ));
                        ctx.state.mark_dirty();
                    }
                }
                ConfirmAction::ResetHard(oid) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::commit::reset_to_commit(
                        &ctx.state.repo.repo,
                        oid,
                        git2::ResetType::Hard,
                    ) {
                        ctx.state
                            .set_flash_message(format!("Erreur reset hard: {}", e));
                    } else {
                        ctx.state.set_flash_message(format!(
                            "Reset hard vers {} effectué ✓",
                            format!("{:.7}", oid)
                        ));
                        ctx.state.mark_dirty();
                    }
                }
                ConfirmAction::StashDrop(index) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) = crate::git::stash::drop_stash(&mut ctx.state.repo.repo, index) {
                        ctx.state
                            .set_flash_message(format!("Erreur suppression stash: {}", e));
                    } else {
                        ctx.state
                            .set_flash_message(format!("Stash @{{{}}} supprimé ✓", index));
                        ctx.state.mark_dirty();
                    }
                }
                ConfirmAction::WorktreeRemove(name) => {
                    ctx.state.pending_confirmation = None;
                    if let Err(e) =
                        crate::git::worktree::remove_worktree(&ctx.state.repo.repo, &name)
                    {
                        ctx.state
                            .set_flash_message(format!("Erreur suppression worktree: {}", e));
                    } else {
                        ctx.state
                            .set_flash_message(format!("Worktree '{}' supprimé ✓", name));
                        ctx.state.mark_dirty();
                    }
                }
                ConfirmAction::CherryPick(oid) => {
                    ctx.state.pending_confirmation = None;
                    match crate::git::commit::cherry_pick_with_result(&ctx.state.repo.repo, oid) {
                        Ok(MergeResult::Success) => {
                            ctx.state.set_flash_message(format!(
                                "Cherry-pick {} effectué ✓",
                                format!("{:.7}", oid)
                            ));
                            ctx.state.mark_dirty();
                        }
                        Ok(MergeResult::Conflicts(conflicts)) => {
                            ctx.state.set_flash_message(format!(
                                "Conflits lors du cherry-pick ({} fichiers)",
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
                                format!("cherry-pick {}", format!("{:.7}", oid)),
                                current,
                                format!("{:.7}", oid),
                            ));
                            ctx.state.view_mode = ViewMode::Conflicts;
                        }
                        Ok(_) => {
                            // UpToDate ou FastForward - ne devrait pas arriver en cherry-pick
                            ctx.state
                                .set_flash_message("Cherry-pick effectué ✓".to_string());
                            ctx.state.mark_dirty();
                        }
                        Err(e) => {
                            ctx.state
                                .set_flash_message(format!("Erreur cherry-pick: {}", e));
                        }
                    }
                }
                ConfirmAction::MergeBranch(source, target) => {
                    ctx.state.pending_confirmation = None;
                    // Note: le merge devrait être fait sur la branche cible,
                    // mais comme on est déjà dessus (par définition), on merge juste la source
                    match crate::git::merge::merge_branch_with_result(&ctx.state.repo.repo, &source)
                    {
                        Ok(MergeResult::UpToDate) => {
                            ctx.state
                                .set_flash_message(format!("Branche '{}' est déjà à jour", source));
                        }
                        Ok(MergeResult::FastForward) => {
                            ctx.state
                                .set_flash_message(format!("Fast-forward vers '{}'", source));
                            ctx.state.mark_dirty();
                        }
                        Ok(MergeResult::Success) => {
                            ctx.state.set_flash_message(format!(
                                "Branche '{}' mergée dans '{}' avec succès",
                                source, target
                            ));
                            ctx.state.mark_dirty();
                        }
                        Ok(MergeResult::Conflicts(conflicts)) => {
                            ctx.state.set_flash_message(format!(
                                "Conflits lors du merge avec '{}' ({} fichiers)",
                                source,
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
                                format!("merge {}", source),
                                current,
                                source.clone(),
                            ));
                            ctx.state.view_mode = ViewMode::Conflicts;
                        }
                        Err(e) => {
                            ctx.state.set_flash_message(format!("Erreur merge: {}", e));
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branch::BranchInfo;
    use crate::git::repo::GitRepo;
    use crate::state::action::{GitAction, NavigationAction, SearchAction};
    use crate::state::{BranchesSection, SelectedBranch};
    use crate::test_utils::ui_driver::UiTestHarness;
    use tempfile::TempDir;

    /// Setup un repo temporaire pour les tests.
    fn setup_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(dir.path(), &opts).unwrap();

        // Configurer git
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        // Commit initial
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(dir.path().to_str().unwrap()).unwrap();
        (dir, git_repo)
    }

    #[test]
    fn test_dispatch_quit_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut dispatcher = ActionDispatcher::new();

        dispatcher.dispatch(&mut state, AppAction::Quit).unwrap();

        assert!(state.should_quit);
    }

    #[test]
    fn test_dispatch_refresh_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.dirty = false;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher.dispatch(&mut state, AppAction::Refresh).unwrap();

        assert!(state.dirty);
    }

    #[test]
    fn test_dispatch_toggle_help() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
        let mut dispatcher = ActionDispatcher::new();

        // Activer l'aide
        dispatcher
            .dispatch(&mut state, AppAction::ToggleHelp)
            .unwrap();
        assert_eq!(state.view_mode, ViewMode::Help);
        assert_eq!(state.previous_view_mode, Some(ViewMode::Graph));

        // Désactiver l'aide
        dispatcher
            .dispatch(&mut state, AppAction::ToggleHelp)
            .unwrap();
        assert_eq!(state.view_mode, ViewMode::Graph);
        assert_eq!(state.previous_view_mode, None);
    }

    #[test]
    fn test_dispatch_switch_view() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::SwitchView(ViewMode::Staging))
            .unwrap();

        assert_eq!(state.view_mode, ViewMode::Staging);
        assert!(state.dirty);
    }

    #[test]
    fn test_dispatch_navigation_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        // Créer un graphe de test avec quelques commits
        state.graph_view.rows = crate::state::selection::ListSelection::with_items(
            (0..5)
                .map(|i| crate::git::graph::GraphRow {
                    node: crate::git::graph::CommitNode {
                        oid: git2::Oid::from_bytes(&[i as u8; 20]).unwrap_or(git2::Oid::zero()),
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
                    connection: None,
                })
                .collect(),
        );
        state.graph_view.rows.select(3);

        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::Navigation(NavigationAction::GoTop))
            .unwrap();

        assert_eq!(state.graph_view.selected_index(), 0);
    }

    #[test]
    fn test_dispatch_search_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut dispatcher = ActionDispatcher::new();

        assert!(!state.search_state.is_active);

        dispatcher
            .dispatch(&mut state, AppAction::Search(SearchAction::Open))
            .unwrap();

        assert!(state.search_state.is_active);
    }

    #[test]
    fn test_dispatch_confirm_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        use crate::ui::confirm_dialog::ConfirmAction;
        state.pending_confirmation = Some(ConfirmAction::DiscardAll);
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::ConfirmAction)
            .unwrap();

        // La confirmation devrait être consommée
        assert!(state.pending_confirmation.is_none());
    }

    #[test]
    fn test_dispatch_cancel_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        use crate::ui::confirm_dialog::ConfirmAction;
        state.pending_confirmation = Some(ConfirmAction::DiscardAll);
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::CancelAction)
            .unwrap();

        assert!(state.pending_confirmation.is_none());
    }

    #[test]
    fn test_dispatch_none_action() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let initial_index = state.graph_view.selected_index();
        let mut dispatcher = ActionDispatcher::new();

        dispatcher.dispatch(&mut state, AppAction::None).unwrap();

        // Aucun changement d'état
        assert_eq!(state.graph_view.selected_index(), initial_index);
    }

    #[test]
    fn test_dispatch_close_branch_panel() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.show_branch_panel = true;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::CloseBranchPanel)
            .unwrap();

        assert!(!state.show_branch_panel);
    }

    #[test]
    fn test_dispatch_switch_bottom_mode() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        use crate::state::BottomLeftMode;
        state.bottom_left_mode = BottomLeftMode::Files;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::SwitchBottomMode)
            .unwrap();

        assert_eq!(state.bottom_left_mode, BottomLeftMode::Parents);
    }

    #[test]
    fn test_dispatch_select_from_bottom_left_opens_diff_panel() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
        state.focus = FocusPanel::BottomLeft;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher.dispatch(&mut state, AppAction::Select).unwrap();

        assert_eq!(state.focus, FocusPanel::BottomRight);
        assert!(!state.graph_view.diff_fullscreen);
    }

    #[test]
    fn test_dispatch_toggle_diff_fullscreen_restores_file_focus_when_closing() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
        state.focus = FocusPanel::BottomLeft;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::ToggleDiffFullscreen)
            .unwrap();
        assert!(state.graph_view.diff_fullscreen);
        assert_eq!(state.focus, FocusPanel::BottomRight);

        dispatcher
            .dispatch(&mut state, AppAction::ToggleDiffFullscreen)
            .unwrap();
        assert!(!state.graph_view.diff_fullscreen);
        assert_eq!(state.focus, FocusPanel::BottomLeft);
    }

    #[test]
    fn test_ui_flow_branch_creation_from_keyboard_input() {
        let mut harness = UiTestHarness::new();

        harness.send_char('3');
        harness.send_char('n');
        harness.send_text("feature/ui-flow");
        harness.send_enter();

        assert_eq!(harness.state.view_mode, ViewMode::Branches);
        assert!(harness.state.branches_view_state.input_action.is_none());
        assert_eq!(
            harness.state.current_flash_message(),
            Some("Branche 'feature/ui-flow' créée ✓")
        );
        assert!(harness
            .state
            .repo
            .repo
            .find_branch("feature/ui-flow", git2::BranchType::Local)
            .is_ok());
    }

    #[test]
    fn test_ui_flow_stash_save_from_branches_input() {
        let mut harness = UiTestHarness::new();
        harness.commit_file("tracked.txt", "base\n", "Add tracked");
        harness.write_file("tracked.txt", "base\nmodifie\n");
        harness.stage_file("tracked.txt");

        harness.send_char('3');
        harness.send_tab();
        harness.send_tab();
        harness.send_char('s');
        harness.send_text("stash ui");
        harness.send_enter();

        let mut stash_count = 0;
        harness
            .state
            .repo
            .repo
            .stash_foreach(|_, message, _| {
                stash_count += 1;
                assert!(message.contains("stash ui"));
                true
            })
            .unwrap();

        assert_eq!(stash_count, 1);
        assert_eq!(
            harness.state.current_flash_message(),
            Some("Stash créé: stash ui ✓")
        );
        assert_eq!(
            harness.state.branches_view_state.focus,
            crate::state::BranchesFocus::List
        );
    }

    #[test]
    fn test_ui_flow_invalid_worktree_input_shows_validation_message() {
        let mut harness = UiTestHarness::new();

        harness.send_char('3');
        harness.send_tab();
        harness.send_char('n');
        harness.send_text("worktree-seul");
        harness.send_enter();

        assert_eq!(harness.state.view_mode, ViewMode::Branches);
        assert_eq!(
            harness.state.current_flash_message(),
            Some("Format: nom chemin [branche]")
        );
        assert!(harness.state.branches_view_state.input_action.is_none());
    }

    #[test]
    fn test_ui_flow_discard_all_confirmation_roundtrip() {
        let mut harness = UiTestHarness::new();
        harness.commit_file("tracked.txt", "base\n", "Add tracked");
        harness.write_file("tracked.txt", "base\nmodifie\n");
        harness.refresh_staging();

        harness.send_char('2');
        harness.send_char('D');
        assert_eq!(
            harness.state.pending_confirmation,
            Some(crate::ui::confirm_dialog::ConfirmAction::DiscardAll)
        );

        harness.send_char('y');

        assert!(harness.state.pending_confirmation.is_none());
        assert_eq!(
            harness.state.current_flash_message(),
            Some("Modifications ignorées ✓")
        );
        let statuses = harness.state.repo.repo.statuses(None).unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_ui_flow_graph_to_files_to_diff_via_keyboard() {
        let mut harness = UiTestHarness::new();
        harness.commit_file("file_a.txt", "alpha\n", "Add alpha");
        harness.commit_file("file_b.txt", "beta\n", "Add beta");
        harness.refresh_graph();

        harness.send_enter();
        assert_eq!(harness.state.focus, FocusPanel::BottomLeft);
        assert!(!harness.state.graph_view.commit_files.is_empty());
        assert!(harness.state.graph_view.selected_file_diff.is_some());

        harness.send_char(' ');
        assert_eq!(harness.state.focus, FocusPanel::BottomRight);
        assert!(!harness.state.graph_view.diff_fullscreen);
    }

    #[test]
    fn test_ui_flow_search_then_filter_clears_search_state() {
        let mut harness = UiTestHarness::new();
        harness.commit_file("first.txt", "a\n", "Fix login bug");
        harness.commit_file("second.txt", "b\n", "Add search feature");
        harness.refresh_graph();

        harness.send_char('/');
        harness.send_text("search");

        assert!(harness.state.search_state.is_active);
        assert_eq!(harness.state.search_state.query, "search");
        assert!(!harness.state.search_state.results.is_empty());

        harness.send_esc();
        harness.send_char('F');
        for _ in 0..4 {
            harness.send_tab();
        }
        harness.send_text("feature");
        harness.send_enter();

        assert_eq!(
            harness.state.graph_filter.message.as_deref(),
            Some("feature")
        );
        assert!(!harness.state.search_state.is_active);
        assert!(harness.state.search_state.results.is_empty());
        assert!(harness.state.search_state.query.is_empty());
        assert_eq!(
            harness.state.current_flash_message(),
            Some("Filtres actifs: message")
        );
    }

    #[test]
    fn test_ui_flow_branches_navigation_reaches_remote_selection() {
        let mut harness = UiTestHarness::new();
        harness.state.view_mode = ViewMode::Branches;
        harness.state.branches_view_state.section = BranchesSection::Branches;
        harness.state.branches_view_state.show_remote = true;
        harness
            .state
            .branches_view_state
            .local_branches
            .set_items(vec![
                BranchInfo::simple("main".to_string(), true, false),
                BranchInfo::simple("feature".to_string(), false, false),
            ]);
        harness
            .state
            .branches_view_state
            .remote_branches
            .set_items(vec![BranchInfo::simple(
                "origin/main".to_string(),
                false,
                true,
            )]);
        harness.state.branches_view_state.selected_branch = Some(SelectedBranch::Local(1));

        harness.send_char('j');

        assert_eq!(
            harness.state.branches_view_state.selected_branch,
            Some(SelectedBranch::Remote(0))
        );
    }
}
