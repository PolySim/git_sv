use super::super::traits::HandlerContext;
use crate::error::Result;
use crate::state::{BranchPickerMode, ViewMode};
use crate::utils::{flash_error, flash_success};

#[derive(Debug, Clone, Copy)]
enum IntegrationMode {
    Merge,
    Rebase,
}

/// Gère la confirmation du merge picker.
pub(super) fn handle_merge_picker_confirm(ctx: &mut HandlerContext) -> Result<()> {
    let selected_branch = ctx
        .state
        .merge_picker
        .as_ref()
        .and_then(|picker| picker.branches.selected_item())
        .cloned();
    let picker_mode = ctx
        .state
        .merge_picker
        .as_ref()
        .map(|picker| picker.mode)
        .unwrap_or(BranchPickerMode::Merge);

    if let Some(branch_name) = selected_branch {
        match picker_mode {
            BranchPickerMode::Compare => {
                if let Err(error) =
                    crate::handler::git::activate_project_tree_comparison(ctx.state, &branch_name)
                {
                    ctx.state
                        .set_flash_message(flash_error("comparaison", error));
                }
            }
            BranchPickerMode::Merge => {
                confirm_branch_integration(ctx, &branch_name, IntegrationMode::Merge)
            }
            BranchPickerMode::Rebase => {
                confirm_branch_integration(ctx, &branch_name, IntegrationMode::Rebase)
            }
        }
    }

    ctx.state.merge_picker = None;
    Ok(())
}

fn confirm_branch_integration(ctx: &mut HandlerContext, branch_name: &str, mode: IntegrationMode) {
    use crate::git::conflict::MergeResult;

    let result = match mode {
        IntegrationMode::Merge => {
            crate::git::merge::merge_branch_with_result(&ctx.state.repo.repo, branch_name)
        }
        IntegrationMode::Rebase => {
            crate::git::rebase::rebase_branch_with_result(&ctx.state.repo.repo, branch_name)
        }
    };

    match result {
        Ok(MergeResult::UpToDate) => {
            let message = match mode {
                IntegrationMode::Merge => format!("Branche '{}' est déjà à jour", branch_name),
                IntegrationMode::Rebase => {
                    format!("Branche courante déjà à jour sur '{}'", branch_name)
                }
            };
            ctx.state.set_flash_message(flash_success(message));
        }
        Ok(MergeResult::FastForward) => {
            let message = match mode {
                IntegrationMode::Merge => format!("Fast-forward vers '{}'", branch_name),
                IntegrationMode::Rebase => format!("Rebase fast-forward sur '{}'", branch_name),
            };
            ctx.state.set_flash_message(flash_success(message));
            ctx.state.mark_dirty();
        }
        Ok(MergeResult::Success) => {
            let message = match mode {
                IntegrationMode::Merge => {
                    format!("Branche '{}' mergée avec succès", branch_name)
                }
                IntegrationMode::Rebase => format!("Rebase effectué sur '{}'", branch_name),
            };
            ctx.state.set_flash_message(flash_success(message));
            ctx.state.mark_dirty();
        }
        Ok(MergeResult::Conflicts(conflicts)) => {
            let message = match mode {
                IntegrationMode::Merge => format!(
                    "Conflits lors du merge avec '{}' ({} fichiers)",
                    branch_name,
                    conflicts.len()
                ),
                IntegrationMode::Rebase => format!(
                    "Conflits lors du rebase sur '{}' ({} fichiers)",
                    branch_name,
                    conflicts.len()
                ),
            };
            ctx.state.set_flash_message(message);
            let current = ctx
                .state
                .current_branch
                .clone()
                .unwrap_or_else(|| "HEAD".to_string());
            let operation = match mode {
                IntegrationMode::Merge => format!("merge {}", branch_name),
                IntegrationMode::Rebase => format!("rebase {}", branch_name),
            };
            ctx.state.conflicts_state = Some(crate::state::ConflictsState::new(
                conflicts,
                operation,
                current,
                branch_name.to_string(),
            ));
            ctx.state.view_mode = ViewMode::Conflicts;
        }
        Err(error) => {
            let operation = match mode {
                IntegrationMode::Merge => "merge",
                IntegrationMode::Rebase => "rebase",
            };
            ctx.state.set_flash_message(flash_error(operation, error));
        }
    }
}

/// Gère le chargement progressif de l'historique.
pub(super) fn handle_load_more_history(ctx: &mut HandlerContext) -> Result<()> {
    use crate::state::MAX_TOTAL_COMMITS;

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
    let target_count = ctx
        .state
        .graph_view
        .target_count_for_next_load()
        .min(MAX_TOTAL_COMMITS);

    if target_count <= current_count {
        ctx.state.graph_view.finish_loading_more();
        ctx.state
            .set_flash_message("Limite d'historique atteinte".to_string());
        return Ok(());
    }

    let load_result = if ctx.state.filters.graph_filter.is_active() {
        load_more_filtered_history(ctx, target_count)
    } else {
        load_more_unfiltered_history(ctx, target_count)
    };

    if let Err(e) = load_result {
        ctx.state.set_flash_message(flash_error("chargement", e));
    }

    // Marquer la fin du chargement
    ctx.state.graph_view.finish_loading_more();

    Ok(())
}

fn load_more_unfiltered_history(ctx: &mut HandlerContext, target_count: usize) -> Result<()> {
    match ctx.state.repo.build_graph_with_more(target_count) {
        Ok((graph, has_more)) => {
            let graph_len = graph.len();

            if graph_len == 0 {
                ctx.state.graph_view.can_load_more = false;
                ctx.state
                    .set_flash_message("Fin de l'historique atteinte".to_string());
            } else {
                ctx.state.replace_graph(graph);

                let total = ctx.state.repo.estimate_total_commits();
                ctx.state
                    .graph_view
                    .update_pagination_state(graph_len, total, has_more);

                let msg = if let Some(total) = total {
                    format!("{} / {} commits chargés", graph_len, total)
                } else {
                    format!("{} commits chargés", graph_len)
                };
                ctx.state.set_flash_message(msg);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn load_more_filtered_history(ctx: &mut HandlerContext, target_count: usize) -> Result<()> {
    match ctx
        .state
        .repo
        .build_graph_filtered_with_more(target_count, &ctx.state.filters.graph_filter)
    {
        Ok((graph, has_more)) => {
            let graph_len = graph.len();

            if graph.is_empty() {
                ctx.state.graph_view.can_load_more = false;
                ctx.state
                    .set_flash_message("Fin de l'historique filtré atteinte".to_string());
            } else {
                ctx.state.replace_graph(graph);

                let total = None;
                ctx.state
                    .graph_view
                    .update_pagination_state(graph_len, total, has_more);

                let msg = format!("{} commits filtrés chargés", graph_len);
                ctx.state.set_flash_message(msg);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Déclenche un chargement supplémentaire quand la sélection approche du bas.
pub fn maybe_load_more_history(state: &mut crate::state::AppState) -> Result<bool> {
    if !matches!(state.view_mode, ViewMode::Graph)
        || state.graph_view.is_empty()
        || !state.graph_view.can_load_more
        || state.graph_view.is_loading_more
    {
        return Ok(false);
    }

    let remaining = state
        .graph_view
        .len()
        .saturating_sub(state.graph_view.selected_index() + 1);

    if remaining > 5 {
        return Ok(false);
    }

    let previous_loaded = state.graph_view.loaded_count;
    let mut ctx = HandlerContext { state };
    handle_load_more_history(&mut ctx)?;

    if ctx.state.graph_view.loaded_count == previous_loaded {
        return Ok(false);
    }

    Ok(true)
}

/// Charge tout l'historique restant jusqu'à la fin.
pub fn load_all_history(state: &mut crate::state::AppState) -> Result<bool> {
    if !matches!(state.view_mode, ViewMode::Graph) || state.graph_view.is_loading_more {
        return Ok(false);
    }

    let mut loaded_any = false;
    while state.graph_view.can_load_more {
        let previous_loaded = state.graph_view.loaded_count;
        let mut ctx = HandlerContext { state };
        handle_load_more_history(&mut ctx)?;

        if ctx.state.graph_view.loaded_count == previous_loaded {
            break;
        }

        loaded_any = true;
    }

    Ok(loaded_any)
}
