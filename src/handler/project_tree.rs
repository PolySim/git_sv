//! Gestion des interactions de la vue arborescence.

use super::edit::edit_text;
use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::{ProjectTreeAction, ProjectTreeFocus};

pub struct ProjectTreeHandler;

impl ActionHandler for ProjectTreeHandler {
    type Action = ProjectTreeAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: ProjectTreeAction) -> Result<()> {
        match action {
            ProjectTreeAction::ToggleSelected => {
                ctx.state.project_tree_state.toggle_selected_directory();
            }
            ProjectTreeAction::ExpandSelected => {
                ctx.state.project_tree_state.expand_selected_directory();
            }
            ProjectTreeAction::CollapseSelected => {
                ctx.state
                    .project_tree_state
                    .collapse_selected_or_select_parent();
            }
            ProjectTreeAction::ActivateTreeEntry(index) => {
                ctx.state.project_tree_state.activate_entry(index);
                ctx.state.project_tree_state.focus = ProjectTreeFocus::Tree;
            }
            ProjectTreeAction::FocusTree => {
                ctx.state.project_tree_state.focus = ProjectTreeFocus::Tree;
            }
            ProjectTreeAction::FocusHistory => {
                ctx.state.project_tree_state.focus = ProjectTreeFocus::History;
                ctx.state.ensure_project_tree_focus_loaded();
            }
            ProjectTreeAction::FocusChangedFiles => {
                ctx.state.project_tree_state.focus = ProjectTreeFocus::ChangedFiles;
                ctx.state.ensure_project_tree_focus_loaded();
            }
            ProjectTreeAction::FocusDiff => {
                ctx.state.project_tree_state.focus = ProjectTreeFocus::Diff;
                ctx.state.ensure_project_tree_focus_loaded();
            }
            ProjectTreeAction::SelectTreeEntry(index) => {
                ctx.state.project_tree_state.select_tree_entry(index);
                ctx.state.project_tree_state.focus = ProjectTreeFocus::Tree;
            }
            ProjectTreeAction::SelectSearchResult(index) => {
                ctx.state.project_tree_state.search.results.select(index);
            }
            ProjectTreeAction::SelectHistoryEntry(index) => {
                ctx.state.project_tree_state.select_history_entry(index);
                ctx.state.project_tree_state.focus = ProjectTreeFocus::History;
                ctx.state.ensure_project_tree_focus_loaded();
            }
            ProjectTreeAction::SelectChangedFile(index) => {
                ctx.state.project_tree_state.select_changed_file(index);
                ctx.state.project_tree_state.focus = ProjectTreeFocus::ChangedFiles;
                ctx.state.ensure_project_tree_focus_loaded();
            }
            ProjectTreeAction::OpenSearch => {
                ctx.state.project_tree_state.open_search();
            }
            ProjectTreeAction::CloseSearch => {
                ctx.state.project_tree_state.close_search();
            }
            ProjectTreeAction::ConfirmSearch => confirm_search(ctx),
            ProjectTreeAction::SearchNext => {
                ctx.state.project_tree_state.search.results.select_next();
            }
            ProjectTreeAction::SearchPrevious => {
                ctx.state
                    .project_tree_state
                    .search
                    .results
                    .select_previous();
            }
            ProjectTreeAction::EditSearch(action) => {
                let search = &mut ctx.state.project_tree_state.search;
                edit_text(
                    &mut search.query,
                    &mut search.cursor,
                    &mut search.selection_anchor,
                    &mut search.edit_history,
                    action,
                );
                ctx.state.project_tree_state.update_search_results();
            }
        }

        Ok(())
    }
}

fn confirm_search(ctx: &mut HandlerContext) {
    let selected_path = ctx
        .state
        .project_tree_state
        .search
        .results
        .selected_item()
        .map(|entry| entry.path.clone());
    let Some(path) = selected_path else {
        return;
    };

    ctx.state.project_tree_state.close_search();
    ctx.state.project_tree_state.reveal_path(&path);
    ctx.state.project_tree_state.focus = ProjectTreeFocus::Tree;
}
