use crate::state::AppState;

pub(super) fn handle_blame_navigation(state: &mut AppState, delta: i32) {
    if let Some(ref mut blame_state) = state.blame_state {
        let line_count = if let Some(ref blame) = blame_state.blame {
            blame.lines.len()
        } else {
            0
        };

        let new_idx = if delta >= 0 {
            (blame_state.selected_line + delta as usize).min(line_count.saturating_sub(1))
        } else {
            blame_state.selected_line.saturating_sub((-delta) as usize)
        };
        blame_state.selected_line = new_idx;
    }
}
