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
    selected_index: usize,
    visible_count: usize,
    loaded_count: usize,
    total_commits: Option<usize>,
    can_load_more: bool,
    is_loading_more: bool,
) -> String {
    let position = if visible_count == 0 {
        0
    } else {
        selected_index.min(visible_count - 1) + 1
    };

    let loading_fr = if is_loading_more {
        " · chargement…".to_string()
    } else if let Some(total) = total_commits.filter(|total| loaded_count < *total) {
        format!(" · {}/{} charges", loaded_count, total)
    } else if can_load_more {
        format!(" · {}+ charges", loaded_count)
    } else {
        String::new()
    };
    let loading_en = if is_loading_more {
        " · loading…".to_string()
    } else if let Some(total) = total_commits.filter(|total| loaded_count < *total) {
        format!(" · {}/{} loaded", loaded_count, total)
    } else if can_load_more {
        format!(" · {}+ loaded", loaded_count)
    } else {
        String::new()
    };

    text_owned(
        format!(
            " Graphe · {} · commit {}/{}{} ",
            branch_name, position, visible_count, loading_fr
        ),
        format!(
            " Graph · {} · commit {}/{}{} ",
            branch_name, position, visible_count, loading_en
        ),
    )
}
