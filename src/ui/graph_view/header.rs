use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::i18n::{text, text_owned};
use crate::ui::theme::current_theme;

pub(super) fn build_empty_state_line(filter_active: bool) -> Line<'static> {
    let theme = current_theme();
    let message = if filter_active {
        text(
            "Aucun commit ne correspond aux filtres actifs.",
            "No commit matches the active filters.",
        )
    } else {
        text("Aucun commit a afficher.", "No commit to display.")
    };

    Line::from(Span::styled(
        message,
        Style::default()
            .fg(theme.text_secondary)
            .add_modifier(Modifier::ITALIC),
    ))
}

pub(super) fn build_title(
    branch_name: &str,
    loaded_count: usize,
    total_commits: Option<usize>,
    can_load_more: bool,
    is_loading_more: bool,
) -> String {
    if is_loading_more {
        return text_owned(
            format!(" Graphe - {} (chargement...) ", branch_name),
            format!(" Graph - {} (loading...) ", branch_name),
        );
    }

    match total_commits {
        Some(total) if total > 0 => {
            if loaded_count >= total {
                text_owned(
                    format!(" Graphe - {} ({} commits) ", branch_name, loaded_count),
                    format!(" Graph - {} ({} commits) ", branch_name, loaded_count),
                )
            } else {
                text_owned(
                    format!(
                        " Graphe - {} ({} / {} commits) ",
                        branch_name, loaded_count, total
                    ),
                    format!(
                        " Graph - {} ({} / {} commits) ",
                        branch_name, loaded_count, total
                    ),
                )
            }
        }
        _ => {
            if can_load_more {
                text_owned(
                    format!(" Graphe - {} ({}+) ", branch_name, loaded_count),
                    format!(" Graph - {} ({}+) ", branch_name, loaded_count),
                )
            } else {
                text_owned(
                    format!(" Graphe - {} ({} commits) ", branch_name, loaded_count),
                    format!(" Graph - {} ({} commits) ", branch_name, loaded_count),
                )
            }
        }
    }
}
