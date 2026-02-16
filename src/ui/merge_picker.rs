//! Widget de sélection de branche pour le merge.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::MergePickerState;

/// Rend le sélecteur de branche pour le merge.
pub fn render(
    frame: &mut Frame,
    state: &MergePickerState,
    current_branch: &Option<String>,
    area: Rect,
) {
    // Calculer la zone centrale pour le popup
    let popup_area = centered_rect(50, 60, area);

    // Effacer la zone sous le popup
    frame.render_widget(Clear, popup_area);

    // Construire le titre avec la branche courante
    let current_branch_name = current_branch.as_deref().unwrap_or("???");
    let title = format!(" Merger dans '{}' ", current_branch_name);

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
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    if !state.branches.is_empty() {
        list_state.select(Some(state.selected));
    }

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    // Rendre la barre d'aide en bas
    render_help_bar(frame, popup_area);
}

/// Rend la barre d'aide du merge picker.
fn render_help_bar(frame: &mut Frame, popup_area: Rect) {
    // Calculer la zone pour la barre d'aide (en dessous du popup)
    let help_area = Rect {
        x: popup_area.x,
        y: popup_area.y + popup_area.height + 1,
        width: popup_area.width,
        height: 1,
    };

    let help_text = "j/k:naviguer  Enter:merger  Esc:annuler";

    let line = Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )]);

    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), help_area);
}

/// Calcule un rectangle centré de dimensions données (en pourcentage).
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
