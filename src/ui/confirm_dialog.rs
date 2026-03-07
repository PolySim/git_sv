//! Composant de dialogue de confirmation pour les actions destructives.

#![allow(dead_code)]

use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Type d'action nécessitant une confirmation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    /// Supprimer une branche
    BranchDelete(String),
    /// Supprimer un worktree
    WorktreeRemove(String),
    /// Supprimer un stash
    StashDrop(usize),
    /// Discard les modifications d'un fichier
    DiscardFile(String),
    /// Discard toutes les modifications
    DiscardAll,
    /// Cherry-pick un commit
    CherryPick(git2::Oid),
    /// Merger une branche (source, cible)
    MergeBranch(String, String),
    /// Avorter le merge en cours
    AbortMerge,
    /// Reset soft vers un commit
    ResetSoft(git2::Oid),
    /// Reset hard vers un commit
    ResetHard(git2::Oid),
}

impl ConfirmAction {
    /// Retourne le message de confirmation pour cette action.
    pub fn message(&self) -> String {
        match self {
            ConfirmAction::BranchDelete(name) => {
                format!("Êtes-vous sûr de vouloir supprimer la branche '{}' ?", name)
            }
            ConfirmAction::WorktreeRemove(name) => {
                format!(
                    "Êtes-vous sûr de vouloir supprimer le worktree '{}' ?",
                    name
                )
            }
            ConfirmAction::StashDrop(index) => {
                format!(
                    "Êtes-vous sûr de vouloir supprimer le stash @{{{}}} ?",
                    index
                )
            }
            ConfirmAction::DiscardFile(path) => {
                format!(
                    "Êtes-vous sûr de vouloir discard les modifications de '{}' ?",
                    path
                )
            }
            ConfirmAction::DiscardAll => {
                "Êtes-vous sûr de vouloir discard TOUTES les modifications non stagées ?"
                    .to_string()
            }
            ConfirmAction::CherryPick(oid) => {
                format!("Êtes-vous sûr de vouloir cherry-pick le commit {oid:.7} ?")
            }
            ConfirmAction::MergeBranch(source, target) => {
                format!("Merger '{}' dans '{}' ?", source, target)
            }
            ConfirmAction::AbortMerge => {
                "Êtes-vous sûr de vouloir avorter le merge en cours ?".to_string()
            }
            ConfirmAction::ResetSoft(oid) => {
                format!(
                    "Reset SOFT vers {oid:.7} ?\nLes modifications seront conservées dans l'index."
                )
            }
            ConfirmAction::ResetHard(oid) => {
                format!("⚠ RESET HARD vers {oid:.7} ?\n\n⚠ ATTENTION : Toutes les modifications non committées seront PERDUES !")
            }
        }
    }

    /// Retourne le titre du dialogue.
    pub fn title(&self) -> &'static str {
        match self {
            ConfirmAction::BranchDelete(_) => "Confirmer la suppression de branche",
            ConfirmAction::WorktreeRemove(_) => "Confirmer la suppression de worktree",
            ConfirmAction::StashDrop(_) => "Confirmer la suppression de stash",
            ConfirmAction::DiscardFile(_) => "Confirmer le discard de fichier",
            ConfirmAction::DiscardAll => "Confirmer le discard de tous les fichiers",
            ConfirmAction::CherryPick(_) => "Confirmer le cherry-pick",
            ConfirmAction::MergeBranch(_, _) => "Confirmer le merge",
            ConfirmAction::AbortMerge => "Confirmer l'annulation du merge",
            ConfirmAction::ResetSoft(_) => "Confirmer le reset soft",
            ConfirmAction::ResetHard(_) => "⚠ Confirmer le reset hard",
        }
    }
}

/// Rend un dialogue de confirmation en overlay.
pub fn render(frame: &mut Frame, action: &ConfirmAction, area: Rect) {
    let theme = current_theme();
    // Calculer la zone centrale pour le popup
    let popup_area = centered_rect(60, 30, area);

    // Effacer la zone sous le popup
    frame.render_widget(Clear, popup_area);

    // Construire le contenu
    let msg = action.message();

    let content = vec![
        Line::from(""),
        Line::from(msg.as_str()),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "y",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Oui  "),
            Span::styled(
                "n",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Non  "),
            Span::styled(
                "ESC",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Annuler"),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(action.title())
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.warning)),
        )
        .style(Style::default().bg(theme.background).fg(theme.text_normal))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}
