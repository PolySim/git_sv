use crate::error::Result;
use crate::state::cache::DiffCacheKey;
use crate::state::{AppState, StagingFocus};

pub fn refresh_staging(state: &mut AppState) -> Result<()> {
    let all_entries = state.repo.status()?;
    refresh_staging_with_entries(state, &all_entries)
}

pub fn refresh_staging_with_entries(
    state: &mut AppState,
    all_entries: &[crate::git::repo::StatusEntry],
) -> Result<()> {
    state.apply_status_entries(all_entries.to_vec());
    load_staging_diff(state);
    Ok(())
}

pub fn load_staging_diff(state: &mut AppState) {
    let selected_file = match state.staging_state.focus {
        StagingFocus::Unstaged => state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected()),
        StagingFocus::Staged => state
            .staging_state
            .staged_files()
            .get(state.staging_state.staged_selected()),
        StagingFocus::Diff => match state.staging_state.last_file_focus {
            StagingFocus::Unstaged => state
                .staging_state
                .unstaged_files()
                .get(state.staging_state.unstaged_selected()),
            StagingFocus::Staged => state
                .staging_state
                .staged_files()
                .get(state.staging_state.staged_selected()),
            _ => None,
        },
        _ => None,
    };

    if let Some(file) = selected_file {
        let cache_key = DiffCacheKey::working_dir(&file.path);

        if let Some(cached_diff) = state.diff_cache.get(&cache_key) {
            state.staging_state.current_diff = Some(cached_diff);
        } else {
            match crate::git::diff::working_dir_file_diff(&state.repo.repo, &file.path) {
                Ok(diff) => {
                    let diff = std::sync::Arc::new(diff);
                    state.diff_cache.put(cache_key, diff.clone());
                    state.staging_state.current_diff = Some(diff);
                    state.staging_state.diff_scroll = 0;
                    state.staging_state.diff_horizontal_offset = 0;
                }
                Err(_) => {
                    state.staging_state.current_diff = None;
                }
            }
        }
    } else {
        state.staging_state.current_diff = None;
    }
}
