//! Handler pour les actions sur les branches.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::BranchAction;
use crate::state::{AppState, BranchesFocus, BranchesSection, SelectedBranch, ViewMode};

/// Handler pour les opérations sur les branches.
pub struct BranchHandler;

impl ActionHandler for BranchHandler {
    type Action = BranchAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: BranchAction) -> Result<()> {
        match action {
            BranchAction::Checkout => handle_checkout(ctx.state),
            BranchAction::Create => handle_create(ctx.state),
            BranchAction::Delete => handle_delete(ctx.state),
            BranchAction::Rename => handle_rename(ctx.state),
            BranchAction::ToggleRemote => handle_toggle_remote(ctx.state),
            BranchAction::Merge => handle_merge(ctx.state),
            BranchAction::StashSave => handle_stash_save(ctx.state),
            BranchAction::StashApply => handle_stash_apply(ctx.state),
            BranchAction::StashPop => handle_stash_pop(ctx.state),
            BranchAction::StashDrop => handle_stash_drop(ctx.state),
            BranchAction::StashFileNext => handle_stash_file_next(ctx.state),
            BranchAction::StashFilePrev => handle_stash_file_prev(ctx.state),
            BranchAction::WorktreeCreate => handle_worktree_create(ctx.state),
            BranchAction::WorktreeRemove => handle_worktree_remove(ctx.state),
            BranchAction::NextSection => handle_next_section(ctx.state),
            BranchAction::PrevSection => handle_prev_section(ctx.state),
            BranchAction::ConfirmInput => handle_confirm_input(ctx.state),
            BranchAction::CancelInput => handle_cancel_input(ctx.state),
            BranchAction::SelectLocalBranch(index) => handle_select_local_branch(ctx.state, index),
            BranchAction::SelectRemoteBranch(index) => {
                handle_select_remote_branch(ctx.state, index)
            }
            BranchAction::SelectWorktree(index) => handle_select_worktree(ctx.state, index),
            BranchAction::SelectStash(index) => handle_select_stash(ctx.state, index),
            BranchAction::FocusList => handle_focus_list(ctx.state),
            BranchAction::FocusDetail => handle_focus_detail(ctx.state),
        }
    }
}

fn handle_select_local_branch(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch = Some(SelectedBranch::Local(index));
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

fn handle_select_remote_branch(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch = Some(SelectedBranch::Remote(index));
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

fn handle_select_worktree(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.set_worktree_selected(index);
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

fn handle_select_stash(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.set_stash_selected(index);
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

fn handle_focus_list(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

fn handle_focus_detail(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Detail;
    }
    Ok(())
}

fn handle_checkout(state: &mut AppState) -> Result<()> {
    let branch_info = if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch_info()
    } else {
        None
    };

    if let Some((branch, selected)) = branch_info {
        // Interdire le checkout sur une branche distante
        if selected.is_remote() {
            state.set_flash_message(
                "Checkout impossible sur une branche distante. Créez d'abord une branche locale."
                    .to_string(),
            );
            return Ok(());
        }

        let branch_name = branch.name.clone();
        match crate::git::branch::checkout_branch(&state.repo.repo, &branch_name) {
            Ok(_) => {
                state.mark_dirty();
                state.set_flash_message(format!("Branche '{}' check-out ✓", branch_name));
            }
            Err(e) => {
                state.set_flash_message(format!("Erreur checkout: {}", e));
            }
        }
    }
    Ok(())
}

fn handle_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = crate::state::BranchesFocus::Input;
        state.branches_view_state.input_action = Some(crate::state::InputAction::CreateBranch);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

fn handle_delete(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected_info = if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch_info()
    } else {
        None
    };

    if let Some((branch, selected)) = selected_info {
        // Interdire la suppression des branches distantes
        if selected.is_remote() {
            state.set_flash_message("Suppression impossible sur une branche distante.".to_string());
            return Ok(());
        }

        // Empêcher la suppression de la branche courante
        if branch.is_head {
            state.set_flash_message("Impossible de supprimer la branche courante".to_string());
            return Ok(());
        }
        let branch_name = branch.name.clone();
        state.pending_confirmation = Some(ConfirmAction::BranchDelete(branch_name));
    }
    Ok(())
}

fn handle_rename(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        // Vérifier si une branche est sélectionnée et si elle est locale
        if let Some((branch, selected)) = state.branches_view_state.selected_branch_info() {
            if selected.is_remote() {
                state.set_flash_message(
                    "Renommage impossible sur une branche distante.".to_string(),
                );
                return Ok(());
            }

            let current_name = branch.name.clone();
            state.branches_view_state.focus = crate::state::BranchesFocus::Input;
            state.branches_view_state.input_action = Some(crate::state::InputAction::RenameBranch);
            state.branches_view_state.input_text = current_name;
            state.branches_view_state.input_cursor = state.branches_view_state.input_text.len();
        }
    }
    Ok(())
}

fn handle_toggle_remote(state: &mut AppState) -> Result<()> {
    state.branches_view_state.toggle_remote();
    Ok(())
}

fn handle_merge(state: &mut AppState) -> Result<()> {
    // Charger la liste des branches pour le merge picker
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();

            // Construire la liste des branches (exclure la branche courante)
            let mut branch_names: Vec<String> = local
                .iter()
                .filter(|b| b.name != current)
                .map(|b| b.name.clone())
                .collect();

            // Ajouter les branches remote
            for b in &remote {
                branch_names.push(b.name.clone());
            }

            if branch_names.is_empty() {
                state.set_flash_message("Aucune autre branche disponible pour merge".to_string());
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}

fn handle_stash_save(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = crate::state::BranchesFocus::Input;
        state.branches_view_state.input_action = Some(crate::state::InputAction::SaveStash);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

fn handle_stash_apply(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::apply_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message("Stash appliqué ✓".to_string());
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_stash_pop(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::pop_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message("Stash pop ✓".to_string());
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_stash_drop(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected) {
            let index = stash.index;
            state.pending_confirmation = Some(ConfirmAction::StashDrop(index));
        }
    }
    Ok(())
}

fn handle_worktree_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = crate::state::BranchesFocus::Input;
        state.branches_view_state.input_action = Some(crate::state::InputAction::CreateWorktree);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

fn handle_worktree_remove(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected = state.branches_view_state.worktree_selected();
    if let Some(worktree) = state.branches_view_state.worktrees.get(selected) {
        let path = worktree.path.clone();
        state.pending_confirmation = Some(ConfirmAction::WorktreeRemove(path));
    }
    Ok(())
}

fn handle_next_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Worktrees,
        BranchesSection::Worktrees => BranchesSection::Stashes,
        BranchesSection::Stashes => BranchesSection::Branches,
    };
    Ok(())
}

fn handle_prev_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Stashes,
        BranchesSection::Worktrees => BranchesSection::Branches,
        BranchesSection::Stashes => BranchesSection::Worktrees,
    };
    Ok(())
}

fn handle_confirm_input(state: &mut AppState) -> Result<()> {
    let input = state.branches_view_state.input_text.trim().to_string();
    if input.is_empty() {
        state.branches_view_state.focus = crate::state::BranchesFocus::List;
        state.branches_view_state.input_action = None;
        return Ok(());
    }

    match state.branches_view_state.input_action {
        Some(crate::state::InputAction::CreateBranch) => {
            match crate::git::branch::create_branch(&state.repo.repo, &input) {
                Ok(_) => {
                    state.set_flash_message(format!("Branche '{}' créée ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(crate::state::InputAction::RenameBranch) => {
            if let Some(branch) = state.branches_view_state.selected_branch() {
                let old_name = branch.name.clone();
                match crate::git::branch::rename_branch(&state.repo.repo, &old_name, &input) {
                    Ok(_) => {
                        state.set_flash_message(format!("Branche renommée → '{}' ✓", input));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
                }
            }
        }
        Some(crate::state::InputAction::SaveStash) => {
            match crate::git::stash::save_stash(&mut state.repo.repo, Some(&input)) {
                Ok(_) => {
                    state.set_flash_message(format!("Stash créé: {} ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(crate::state::InputAction::CreateWorktree) => {
            // Le format attendu est "nom chemin [branche]"
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() < 2 {
                state.set_flash_message("Format: nom chemin [branche]".to_string());
            } else {
                let name = parts[0];
                let path = parts[1];
                let branch = parts.get(2).copied();

                // Validation : nom non vide
                if name.is_empty() {
                    state.set_flash_message(
                        "Erreur: le nom du worktree ne peut pas être vide".to_string(),
                    );
                } else if path.is_empty() {
                    state.set_flash_message(
                        "Erreur: le chemin du worktree ne peut pas être vide".to_string(),
                    );
                } else {
                    // Vérifier si un worktree avec ce nom existe déjà
                    let worktree_exists = state
                        .branches_view_state
                        .worktrees
                        .iter()
                        .any(|w| w.name == name);

                    if worktree_exists {
                        state.set_flash_message(format!(
                            "Erreur: un worktree '{}' existe déjà",
                            name
                        ));
                    } else {
                        match crate::git::worktree::create_worktree(
                            &state.repo.repo,
                            name,
                            path,
                            branch,
                        ) {
                            Ok(_) => {
                                state.set_flash_message(format!("Worktree '{}' créé ✓", name));
                                state.mark_dirty();

                                // Recharger la liste des worktrees
                                if let Ok(worktrees) =
                                    crate::git::worktree::list_worktrees(&state.repo.repo)
                                {
                                    state.branches_view_state.worktrees.set_items(worktrees);

                                    // Tenter de resélectionner le worktree créé
                                    if let Some(idx) = state
                                        .branches_view_state
                                        .worktrees
                                        .iter()
                                        .position(|w| w.name == name)
                                    {
                                        state.branches_view_state.worktrees.select(idx);
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("{}", e);
                                if error_msg.contains("exists") || error_msg.contains("déjà") {
                                    state.set_flash_message(format!(
                                        "Erreur: le chemin '{}' existe déjà",
                                        path
                                    ));
                                } else if error_msg.contains("invalid")
                                    || error_msg.contains("invalide")
                                {
                                    state.set_flash_message(format!(
                                        "Erreur: chemin invalide '{}'",
                                        path
                                    ));
                                } else if error_msg.contains("branch")
                                    || error_msg.contains("branche")
                                {
                                    state.set_flash_message(format!(
                                        "Erreur: branche '{}' inexistante",
                                        branch.unwrap_or("")
                                    ));
                                } else {
                                    state.set_flash_message(format!(
                                        "Erreur création worktree: {}",
                                        e
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {}
    }

    state.branches_view_state.focus = crate::state::BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}

fn handle_cancel_input(state: &mut AppState) -> Result<()> {
    state.branches_view_state.focus = crate::state::BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}

fn handle_stash_file_next(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        if let Some(stash) = state.branches_view_state.stashes.selected_item() {
            let file_count = stash.files.len();
            if file_count > 0 {
                let idx = &mut state.branches_view_state.stash_file_selected;
                *idx = (*idx + 1).min(file_count - 1);
                // Charger le diff du fichier sélectionné
                load_stash_file_diff(state)?;
            }
        }
    }
    Ok(())
}

fn handle_stash_file_prev(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        let idx = &mut state.branches_view_state.stash_file_selected;
        *idx = idx.saturating_sub(1);
        load_stash_file_diff(state)?;
    }
    Ok(())
}

pub fn load_stash_file_diff(state: &mut AppState) -> Result<()> {
    if let Some(stash) = state.branches_view_state.stashes.selected_item() {
        let idx = state.branches_view_state.stash_file_selected;
        if let Some(file) = stash.files.get(idx) {
            match state.repo.stash_file_diff(stash.oid, &file.path) {
                Ok(diff) => {
                    state.branches_view_state.stash_file_diff = Some(diff);
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur chargement diff: {}", e));
                    state.branches_view_state.stash_file_diff = None;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::{BranchesFocus, BranchesSection, InputAction};
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
    fn test_handle_next_section() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        // Test cycle forward
        state.branches_view_state.section = BranchesSection::Branches;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(
            state.branches_view_state.section,
            BranchesSection::Worktrees
        );

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Stashes);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Branches);
    }

    #[test]
    fn test_handle_prev_section() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        // Test cycle backward
        state.branches_view_state.section = BranchesSection::Branches;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Stashes);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(
            state.branches_view_state.section,
            BranchesSection::Worktrees
        );

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Branches);
    }

    #[test]
    fn test_handle_toggle_remote() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        assert!(!state.branches_view_state.show_remote);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ToggleRemote)
                .unwrap();
        }
        assert!(state.branches_view_state.show_remote);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ToggleRemote)
                .unwrap();
        }
        assert!(!state.branches_view_state.show_remote);
    }

    #[test]
    fn test_handle_create_branch() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::Create).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::CreateBranch)
        );
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_cancel_input() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.branches_view_state.input_text = "test-branch".to_string();
        state.branches_view_state.input_cursor = 5;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::CancelInput).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::List);
        assert!(state.branches_view_state.input_action.is_none());
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_list_in_graph_view() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
        let mut handler = BranchHandler;
    }

    #[test]
    fn test_handle_stash_save() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::StashSave).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::SaveStash)
        );
        assert!(state.branches_view_state.input_text.is_empty());
    }

    #[test]
    fn test_handle_worktree_create_opens_input() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.section = BranchesSection::Worktrees;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::WorktreeCreate)
                .unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::CreateWorktree)
        );
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_confirm_input_create_worktree_validation_empty() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateWorktree);
        state.branches_view_state.input_text = "".to_string();
        state.branches_view_state.input_cursor = 0;

        // Simuler la confirmation avec un input vide
        let mut handler = BranchHandler;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ConfirmInput)
                .unwrap();
        }

        // L'input vide devrait annuler l'opération
        assert_eq!(state.branches_view_state.focus, BranchesFocus::List);
        assert!(state.branches_view_state.input_action.is_none());
    }
}
