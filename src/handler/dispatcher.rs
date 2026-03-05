//! Dispatcher principal pour router les actions vers les handlers appropriés.
//!
//! Ce module remplace la logique monolithique de event.rs par un système
//! modulaire où chaque type d'action est géré par un handler spécialisé.

use crate::error::Result;
use crate::state::action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
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
                ctx.state.diff_view_mode.toggle();
                // Aussi toggle le mode dans la vue staging si on y est.
                ctx.state.staging_state.diff_view_mode.toggle();
                Ok(())
            }

            // Toggle diff fullscreen mode
            AppAction::ToggleDiffFullscreen => {
                ctx.state.diff_fullscreen = !ctx.state.diff_fullscreen;
                // Réinitialiser le scroll horizontal quand on bascule
                if !ctx.state.diff_fullscreen {
                    ctx.state.diff_horizontal_offset = 0;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::action::{GitAction, NavigationAction, SearchAction};
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
        state.selected_index = 5;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher
            .dispatch(&mut state, AppAction::Navigation(NavigationAction::GoTop))
            .unwrap();

        assert_eq!(state.selected_index, 0);
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
        let initial_state = state.selected_index;
        let mut dispatcher = ActionDispatcher::new();

        dispatcher.dispatch(&mut state, AppAction::None).unwrap();

        // Aucun changement d'état
        assert_eq!(state.selected_index, initial_state);
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
}
