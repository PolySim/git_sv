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
mod clipboard;
mod confirmations;
mod pickers;
#[cfg(test)]
mod tests;

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
                ctx.state.request_quit();
                Ok(())
            }

            AppAction::Refresh => {
                ctx.state.schedule_refresh();
                Ok(())
            }

            AppAction::ToggleHelp => {
                ctx.state.toggle_help();
                Ok(())
            }

            AppAction::SwitchBottomMode => {
                ctx.state.bottom_left_mode.toggle();
                Ok(())
            }

            AppAction::SwitchView(view_mode) => {
                ctx.state.enter_view(view_mode);
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
                clipboard::handle_copy_to_clipboard(&mut ctx)
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

            AppAction::MergePickerConfirm => pickers::handle_merge_picker_confirm(&mut ctx),

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
                        ctx.state.open_confirmation(
                            crate::ui::confirm_dialog::ConfirmAction::ResetSoft(oid),
                        );
                    } else {
                        ctx.state.open_confirmation(
                            crate::ui::confirm_dialog::ConfirmAction::ResetHard(oid),
                        );
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
            AppAction::ConfirmAction => confirmations::handle_confirm_action(&mut ctx),
            AppAction::CancelAction => {
                ctx.state.close_confirmation();
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
            AppAction::LoadMoreHistory => pickers::handle_load_more_history(&mut ctx),

            // Aucune action
            AppAction::None => Ok(()),
        }
    }
}

pub(crate) fn maybe_load_more_history(state: &mut AppState) -> Result<bool> {
    pickers::maybe_load_more_history(state)
}

pub(crate) fn load_all_history(state: &mut AppState) -> Result<bool> {
    pickers::load_all_history(state)
}

impl Default for ActionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
