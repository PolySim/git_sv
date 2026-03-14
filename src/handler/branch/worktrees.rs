use crate::error::Result;
use crate::state::AppState;

pub(super) fn handle_worktree_remove(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected = state.branches_view_state.worktree_selected();
    if let Some(worktree) = state.branches_view_state.worktrees.get(selected) {
        let path = worktree.path.clone();
        state.open_confirmation(ConfirmAction::WorktreeRemove(path));
    }
    Ok(())
}
