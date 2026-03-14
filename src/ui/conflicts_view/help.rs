use crate::ui::common::centered_rect;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::current_theme;

pub struct ConflictsHelpOverlayRenderContext {
    pub area: Rect,
}

pub fn render_help_overlay(frame: &mut Frame, ctx: ConflictsHelpOverlayRenderContext) {
    let ConflictsHelpOverlayRenderContext { area } = ctx;

    let theme = current_theme();
    let popup_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, popup_area);

    let content = vec![
        Line::from(vec![Span::styled(
            "Raccourcis de la vue Conflits",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  ↑/↓ ou j/k  - Naviguer (fichiers / sections / lignes selon le panneau)"),
        Line::from("  Tab         - Panneau suivant (Fichiers → Ours → Theirs → Résultat)"),
        Line::from("  Shift+Tab   - Panneau précédent"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Résolution",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  o           - Garder la version 'ours' (HEAD)"),
        Line::from("  t           - Garder la version 'theirs' (branche mergée)"),
        Line::from("  b           - Garder les deux versions (mode Bloc uniquement)"),
        Line::from("  Espace      - Choisir/déchoisir un bloc ou une ligne"),
        Line::from("  Enter       - Valider et sauvegarder le fichier courant"),
        Line::from("  r           - Marquer comme résolu (depuis la liste de fichiers)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Édition du résultat",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  i ou e      - Entrer en mode édition (panneau Résultat)"),
        Line::from("  Esc         - Quitter le mode édition"),
        Line::from("  ↑/↓/←/→     - Déplacer le curseur"),
        Line::from("  Caractères  - Insérer du texte"),
        Line::from("  Backspace   - Supprimer le caractère avant"),
        Line::from("  Delete      - Supprimer le caractère sous le curseur"),
        Line::from("  Enter       - Insérer une nouvelle ligne"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions globales",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  F/B/L       - Mode Fichier/Bloc/Ligne (touche directe)"),
        Line::from("  V           - Finaliser le merge (créer le commit)"),
        Line::from("  q ou Esc    - Annuler le merge et revenir au graph"),
        Line::from("  1/2/3       - Basculer vers Graph/Staging/Branches"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Appuyez sur ? pour fermer cette aide",
            Style::default().fg(theme.text_secondary),
        )]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title("Aide - Résolution de conflits")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}
