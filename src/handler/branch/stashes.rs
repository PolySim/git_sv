use crate::error::Result;
use crate::state::{AppState, BranchesSection, ViewMode};
use crate::utils::{flash_error_message, flash_success};

pub(super) fn handle_stash_apply(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            if !selected_stash_is_current(state, index, stash.oid) {
                return Ok(());
            }
            let result = crate::git::stash::apply_stash(&mut state.repo.repo, index);
            // Un apply refusé peut néanmoins avoir écrit des conflits dans l'index.
            state.mark_dirty();
            match result {
                Ok(_) => {
                    state.set_flash_message(flash_success("Stash appliqué"));
                }
                Err(e) => {
                    state.set_flash_message(flash_error_message(e));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_stash_pop(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            if !selected_stash_is_current(state, index, stash.oid) {
                return Ok(());
            }
            let result = crate::git::stash::pop_stash(&mut state.repo.repo, index);
            state.mark_dirty();
            match result {
                Ok(_) => {
                    state.set_flash_message(flash_success("Stash pop"));
                }
                Err(e) => {
                    state.set_flash_message(flash_error_message(e));
                }
            }
        }
    }
    Ok(())
}

fn selected_stash_is_current(state: &mut AppState, index: usize, oid: git2::Oid) -> bool {
    let matches = state
        .repo
        .repo
        .revparse_single(&format!("stash@{{{index}}}"))
        .is_ok_and(|stash| stash.id() == oid);
    if !matches {
        state.set_flash_message(
            "La liste des stashes a changé ; sélectionnez à nouveau le stash à appliquer",
        );
        state.mark_dirty();
    }
    matches
}

pub(super) fn handle_stash_drop(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected) {
            state.open_confirmation(ConfirmAction::StashDrop {
                index: stash.index,
                oid: stash.oid,
            });
        }
    }
    Ok(())
}

pub(super) fn handle_stash_file_next(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        if let Some(stash) = state.branches_view_state.stashes.selected_item() {
            let file_count = stash.files.len();
            if file_count > 0 {
                let idx = &mut state.branches_view_state.stash_file_selected;
                *idx = (*idx + 1).min(file_count - 1);
                load_stash_file_diff(state)?;
            }
        }
    }
    Ok(())
}

pub(super) fn handle_stash_file_prev(state: &mut AppState) -> Result<()> {
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
    state.branches_view_state.stash_file_diff = None;
    state.branches_view_state.stash_diff_scroll = 0;
    if let Some(stash) = state.branches_view_state.stashes.selected_item() {
        let idx = state.branches_view_state.stash_file_selected;
        if let Some(file) = stash.files.get(idx) {
            match state.repo.stash_file_diff(stash.oid, &file.path) {
                Ok(diff) => {
                    state.branches_view_state.stash_file_diff = Some(diff);
                }
                Err(e) => {
                    state.set_flash_message(crate::utils::flash_error("chargement diff", e));
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
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn stash_actions_ignore_hidden_stash_selection() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "base");
        let mut state = AppState::new(
            crate::git::GitRepo { repo },
            temp.path().display().to_string(),
        )
        .unwrap();
        std::fs::write(temp.path().join("file.txt"), "saved\n").unwrap();
        crate::git::stash::save_stash(&mut state.repo.repo, Some("selected")).unwrap();
        state.enter_view(ViewMode::Branches);
        state.refresh_from_repo().unwrap();
        let saved = state.repo.repo.refname_to_id("refs/stash").unwrap();
        for section in [BranchesSection::Branches, BranchesSection::Worktrees] {
            state.branches_view_state.section = section;
            handle_stash_apply(&mut state).unwrap();
            handle_stash_pop(&mut state).unwrap();
            handle_stash_drop(&mut state).unwrap();
            assert_eq!(state.repo.repo.refname_to_id("refs/stash").unwrap(), saved);
            assert_eq!(
                std::fs::read(temp.path().join("file.txt")).unwrap(),
                b"base\n"
            );
            assert!(state.ui.pending_confirmation.is_none());
        }
    }

    #[test]
    fn stale_stash_selection_cannot_apply_or_pop_a_different_stash() {
        for pop in [false, true] {
            let (temp, repo) = create_test_repo();
            commit_file(&repo, "file.txt", "base\n", "base");
            let mut state = AppState::new(
                crate::git::GitRepo { repo },
                temp.path().display().to_string(),
            )
            .unwrap();
            std::fs::write(temp.path().join("file.txt"), "selected stash\n").unwrap();
            crate::git::stash::save_stash(&mut state.repo.repo, Some("selected")).unwrap();
            state.enter_view(ViewMode::Branches);
            state.refresh_from_repo().unwrap();
            state.branches_view_state.section = BranchesSection::Stashes;
            let selected = state.branches_view_state.stashes.first().unwrap().oid;
            std::fs::write(temp.path().join("file.txt"), "new stash\n").unwrap();
            crate::git::stash::save_stash(&mut state.repo.repo, Some("new")).unwrap();
            if pop {
                handle_stash_pop(&mut state).unwrap();
            } else {
                handle_stash_apply(&mut state).unwrap();
            }
            assert_eq!(
                std::fs::read(temp.path().join("file.txt")).unwrap(),
                b"base\n"
            );
            let stashes = crate::git::stash::list_stashes(&mut state.repo.repo).unwrap();
            assert_eq!(stashes.len(), 2);
            assert!(stashes.iter().any(|stash| stash.oid == selected));
            assert!(state.dirty);
        }
    }
}
