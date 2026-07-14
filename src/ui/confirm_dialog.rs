//! Composant de dialogue de confirmation pour les actions destructives.

use crate::i18n::{text, text_owned};
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct ConfirmDialogRenderContext<'a> {
    pub action: &'a ConfirmAction,
    pub area: Rect,
}

/// Type d'action nécessitant une confirmation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    /// Supprimer une branche
    BranchDelete(String),
    /// Supprimer un tag.
    TagDelete(String),
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
    /// Avorter le merge en cours
    AbortMerge,
    /// Reset soft vers un commit
    ResetSoft(git2::Oid),
    /// Reset mixed vers un commit
    ResetMixed(git2::Oid),
    /// Reset hard vers un commit
    ResetHard(git2::Oid),
    /// Ouvrir l'éditeur Git pour un rebase interactif.
    InteractiveRebase(git2::Oid),
    /// Revenir à l'ancien HEAD conservé par le reflog.
    UndoLastOperation(git2::Oid, String),
    /// Démarrer un bisect entre un commit bon et HEAD.
    BisectStart { good: git2::Oid, bad: git2::Oid },
}

impl ConfirmAction {
    /// Retourne le message de confirmation pour cette action.
    pub fn message(&self) -> String {
        match self {
            ConfirmAction::BranchDelete(name) => {
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer la branche '{}' ?", name),
                    format!("Are you sure you want to delete branch '{}' ?", name),
                )
            }
            ConfirmAction::TagDelete(name) => text_owned(
                format!("Supprimer le tag '{}' ?", name),
                format!("Delete tag '{}' ?", name),
            ),
            ConfirmAction::WorktreeRemove(name) => {
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer le worktree '{}' ?", name),
                    format!("Are you sure you want to delete worktree '{}' ?", name),
                )
            }
            ConfirmAction::StashDrop(index) => {
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer le stash @{{{}}} ?", index),
                    format!("Are you sure you want to delete stash @{{{}}} ?", index),
                )
            }
            ConfirmAction::DiscardFile(path) => {
                text_owned(
                    format!("Etes-vous sur de vouloir abandonner les modifications de '{}' ?", path),
                    format!("Are you sure you want to discard changes in '{}' ?", path),
                )
            }
            ConfirmAction::DiscardAll => {
                text_owned(
                    "Etes-vous sur de vouloir abandonner TOUTES les modifications non indexees ?",
                    "Are you sure you want to discard ALL unstaged changes?",
                )
            }
            ConfirmAction::CherryPick(oid) => text_owned(
                format!("Etes-vous sur de vouloir cherry-pick le commit {oid:.7} ?"),
                format!("Are you sure you want to cherry-pick commit {oid:.7} ?"),
            ),
            ConfirmAction::AbortMerge => {
                text_owned(
                    "Etes-vous sur de vouloir annuler la fusion en cours ?",
                    "Are you sure you want to abort the current merge?",
                )
            }
            ConfirmAction::ResetSoft(oid) => {
                text_owned(
                    format!(
                        "Reset SOFT vers {oid:.7} ?\nLes modifications seront conservees dans l'index."
                    ),
                    format!(
                        "SOFT reset to {oid:.7} ?\nChanges will be kept in the index."
                    ),
                )
            }
            ConfirmAction::ResetMixed(oid) => {
                text_owned(
                    format!(
                        "Reset MIXED vers {oid:.7} ?\nLes modifications seront conservees dans le working tree mais retirees de l'index."
                    ),
                    format!(
                        "MIXED reset to {oid:.7} ?\nChanges will be kept in the working tree but removed from the index."
                    ),
                )
            }
            ConfirmAction::ResetHard(oid) => {
                text_owned(
                    format!("⚠ RESET HARD vers {oid:.7} ?\n\n⚠ ATTENTION : Toutes les modifications non committees seront PERDUES !"),
                    format!("⚠ HARD RESET to {oid:.7} ?\n\n⚠ WARNING: All uncommitted changes will be LOST!"),
                )
            }
            ConfirmAction::InteractiveRebase(oid) => text_owned(
                format!(
                    "Ouvrir le rebase interactif a partir de {oid:.7} ?\nLa TUI sera suspendue pendant l'edition."
                ),
                format!(
                    "Open interactive rebase from {oid:.7}?\nThe TUI will be suspended while editing."
                ),
            ),
            ConfirmAction::UndoLastOperation(oid, description) => text_owned(
                format!(
                    "Annuler '{}' et revenir a {oid:.7} ?\nLes differences seront conservees dans le working tree.",
                    description
                ),
                format!(
                    "Undo '{}' and return to {oid:.7}?\nDifferences will remain in the working tree.",
                    description
                ),
            ),
            ConfirmAction::BisectStart { good, bad } => text_owned(
                format!(
                    "Démarrer un bisect entre le commit bon {good:.7} et HEAD {bad:.7} ?\nLe working tree changera de commit pendant la recherche."
                ),
                format!(
                    "Start bisect between good commit {good:.7} and HEAD {bad:.7}?\nThe working tree will change commits during the search."
                ),
            ),
        }
    }

    /// Retourne le titre du dialogue.
    pub fn title(&self) -> &'static str {
        match self {
            ConfirmAction::BranchDelete(_) => {
                text("Confirmer suppression branche", "Confirm branch deletion")
            }
            ConfirmAction::TagDelete(_) => {
                text("Confirmer suppression tag", "Confirm tag deletion")
            }
            ConfirmAction::WorktreeRemove(_) => text(
                "Confirmer suppression worktree",
                "Confirm worktree deletion",
            ),
            ConfirmAction::StashDrop(_) => {
                text("Confirmer suppression stash", "Confirm stash deletion")
            }
            ConfirmAction::DiscardFile(_) => {
                text("Confirmer abandon fichier", "Confirm file discard")
            }
            ConfirmAction::DiscardAll => text(
                "Confirmer abandon de tous les fichiers",
                "Confirm discard of all files",
            ),
            ConfirmAction::CherryPick(_) => text("Confirmer le cherry-pick", "Confirm cherry-pick"),
            ConfirmAction::AbortMerge => {
                text("Confirmer l'annulation de la fusion", "Confirm merge abort")
            }
            ConfirmAction::ResetSoft(_) => text("Confirmer le reset soft", "Confirm soft reset"),
            ConfirmAction::ResetMixed(_) => text("Confirmer le reset mixed", "Confirm mixed reset"),
            ConfirmAction::ResetHard(_) => {
                text("⚠ Confirmer le reset hard", "⚠ Confirm hard reset")
            }
            ConfirmAction::InteractiveRebase(_) => text(
                "Confirmer le rebase interactif",
                "Confirm interactive rebase",
            ),
            ConfirmAction::UndoLastOperation(_, _) => {
                text("Confirmer l'annulation", "Confirm undo")
            }
            ConfirmAction::BisectStart { .. } => text("Confirmer le bisect", "Confirm bisect"),
        }
    }
}

/// Rend un dialogue de confirmation en overlay.
pub fn render(frame: &mut Frame, ctx: ConfirmDialogRenderContext<'_>) {
    let ConfirmDialogRenderContext { action, area } = ctx;

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
            Span::raw(text(" - Oui  ", " - Yes  ")),
            Span::styled(
                "n",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text(" - Non  ", " - No  ")),
            Span::styled(
                "ESC",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text(" - Annuler", " - Cancel")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{with_language, Language};

    #[test]
    fn test_confirm_title_is_localized_in_english() {
        with_language(Language::En, || {
            let title = ConfirmAction::BranchDelete("feature".to_string()).title();
            assert_eq!(title, "Confirm branch deletion");
        });
    }
}
