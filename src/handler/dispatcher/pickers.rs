use crate::error::Result;
use crate::state::ViewMode;

use super::super::traits::HandlerContext;

/// Gère la confirmation du merge picker.
pub(super) fn handle_merge_picker_confirm(ctx: &mut HandlerContext) -> Result<()> {
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
pub(super) fn handle_load_more_history(ctx: &mut HandlerContext) -> Result<()> {
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
