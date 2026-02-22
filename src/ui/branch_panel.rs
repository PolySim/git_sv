use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::git::branch::BranchInfo;
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;

/// Rend le panneau de branches en overlay.
pub fn render(frame: &mut Frame, branches: &[BranchInfo], branch_selected: usize, area: Rect) {
    let theme = current_theme();

    // Créer une zone centrale pour le popup (60% largeur, 50% hauteur).
    let popup_area = centered_rect(60, 50, area);

    // Effacer l'arrière-plan derrière le popup.
    frame.render_widget(Clear, popup_area);

    // Construire la liste des branches.
    let items: Vec<ListItem> = branches
        .iter()
        .enumerate()
        .map(|(i, branch)| build_branch_line(branch, i == branch_selected))
        .collect();

    // Titre avec le nombre de branches.
    let title = format!(" Branches ({}) ", branches.len());

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_active)),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(
        list,
        popup_area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(branch_selected)),
    );

    // Barre d'aide en bas du panneau.
    render_help_bar(frame, popup_area);
}

/// Construit une ligne pour une branche.
fn build_branch_line(branch: &BranchInfo, is_selected: bool) -> ListItem<'static> {
    let theme = current_theme();
    let prefix = if branch.is_head { "* " } else { "  " };
    let name = &branch.name;

    let style = if branch.is_head {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default()
            .bg(theme.selection_bg)
            .fg(theme.selection_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_normal)
    };

    let line = Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(name.clone(), style),
    ]);

    ListItem::new(line)
}

/// Rend la barre d'aide en bas du panneau.
fn render_help_bar(frame: &mut Frame, area: Rect) {
    let theme = current_theme();
    // Créer une zone pour la barre d'aide (dernière ligne du popup).
    let help_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(2),
        height: 1,
    };

    let help_text = "Enter:checkout  n:new  d:delete  Esc/b:close";
    let paragraph = Paragraph::new(help_text).style(Style::default().fg(theme.text_secondary));

    frame.render_widget(paragraph, help_area);
}
