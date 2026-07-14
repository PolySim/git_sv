//! Widget de sélection de branche pour le merge.

use crate::i18n::{text, text_owned};
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::{BranchPickerMode, MergePickerState};

pub struct MergePickerRenderContext<'a> {
    pub state: &'a MergePickerState,
    pub current_branch: Option<&'a str>,
    pub area: Rect,
}

/// Rend le sélecteur de branche pour le merge.
pub fn render(frame: &mut Frame, ctx: MergePickerRenderContext<'_>) {
    let MergePickerRenderContext {
        state,
        current_branch,
        area,
    } = ctx;

    let theme = current_theme();
    // Calculer la zone centrale pour le popup
    let popup_area = centered_rect(50, 60, area);

    // Effacer la zone sous le popup
    frame.render_widget(Clear, popup_area);

    // Construire le titre avec la branche courante
    let current_branch_name = current_branch.unwrap_or("???");
    let title = match state.mode {
        BranchPickerMode::Merge => text_owned(
            format!(" Fusionner dans '{}' ", current_branch_name),
            format!(" Merge into '{}' ", current_branch_name),
        ),
        BranchPickerMode::Rebase => text_owned(
            format!(" Rebase '{}' sur ", current_branch_name),
            format!(" Rebase '{}' onto ", current_branch_name),
        ),
        BranchPickerMode::Compare => text_owned(
            format!(" Comparer '{}' avec ", current_branch_name),
            format!(" Compare '{}' with ", current_branch_name),
        ),
    };

    // Construire la liste des branches
    let items: Vec<ListItem> = state
        .branches
        .iter()
        .map(|branch| {
            let style = Style::default();
            let line = Line::from(vec![Span::raw("  "), Span::styled(branch, style)]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.warning)),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    if !state.branches.is_empty() {
        list_state.select(Some(state.selected()));
    }

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    // Rendre la barre d'aide en bas
    render_help_bar(frame, popup_area, state.mode);
}

/// Rend la barre d'aide du merge picker.
fn render_help_bar(frame: &mut Frame, popup_area: Rect, mode: BranchPickerMode) {
    let theme = current_theme();
    // Calculer la zone pour la barre d'aide (en dessous du popup)
    let help_area = Rect {
        x: popup_area.x,
        y: popup_area.y + popup_area.height + 1,
        width: popup_area.width,
        height: 1,
    };

    let help_text = match mode {
        BranchPickerMode::Merge => text(
            "j/k:naviguer  Enter:fusionner  Esc:annuler",
            "j/k:navigate  Enter:merge  Esc:cancel",
        ),
        BranchPickerMode::Rebase => text(
            "j/k:naviguer  Enter:rebase  Esc:annuler",
            "j/k:navigate  Enter:rebase  Esc:cancel",
        ),
        BranchPickerMode::Compare => text(
            "j/k:naviguer  Enter:comparer  Esc:annuler",
            "j/k:navigate  Enter:compare  Esc:cancel",
        ),
    };

    let line = Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(theme.text_secondary),
    )]);

    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), help_area);
}
