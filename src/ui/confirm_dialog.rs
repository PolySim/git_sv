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
    StashDrop { index: usize, oid: git2::Oid },
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
    /// Exécuter une commande utilisateur.
    CustomCommand(crate::config::CustomCommandConfig),
}

impl ConfirmAction {
    /// Retourne le message de confirmation pour cette action.
    pub fn message(&self) -> String {
        match self {
            ConfirmAction::BranchDelete(name) => {
                let name = crate::utils::terminal_text(name);
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer la branche '{}' ?", name),
                    format!("Are you sure you want to delete branch '{}' ?", name),
                )
            }
            ConfirmAction::TagDelete(name) => text_owned(
                format!("Supprimer le tag '{}' ?", crate::utils::terminal_text(name)),
                format!("Delete tag '{}' ?", crate::utils::terminal_text(name)),
            ),
            ConfirmAction::WorktreeRemove(name) => {
                let name = crate::utils::terminal_path(name);
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer le worktree '{}' ?", name),
                    format!("Are you sure you want to delete worktree '{}' ?", name),
                )
            }
            ConfirmAction::StashDrop { index, .. } => {
                text_owned(
                    format!("Etes-vous sur de vouloir supprimer le stash @{{{}}} ?", index),
                    format!("Are you sure you want to delete stash @{{{}}} ?", index),
                )
            }
            ConfirmAction::DiscardFile(path) => {
                let path = crate::utils::terminal_path(path);
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
                    crate::utils::terminal_text(description)
                ),
                format!(
                    "Undo '{}' and return to {oid:.7}?\nDifferences will remain in the working tree.",
                    crate::utils::terminal_text(description)
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
            ConfirmAction::CustomCommand(command) => text_owned(
                format!(
                    "Exécuter la commande '{}' ?\n\n{}",
                    crate::utils::terminal_text(&command.name), crate::utils::terminal_text(&command.command)
                ),
                format!("Run command '{}'?\n\n{}", crate::utils::terminal_text(&command.name), crate::utils::terminal_text(&command.command)),
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
            ConfirmAction::StashDrop { .. } => {
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
            ConfirmAction::CustomCommand(_) => text("Confirmer la commande", "Confirm command"),
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
    let hints = confirmation_hints();

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
            Span::raw(hints[0]),
            Span::styled(
                "n",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(hints[1]),
            Span::styled(
                "ESC",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(hints[2]),
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

fn confirmation_hints() -> [&'static str; 3] {
    [
        text(" - Oui  ", " - Yes  "),
        text(" - Non  ", " - No  "),
        text(" - Annuler", " - Cancel"),
    ]
}

/// Zone exacte du bouton affirmatif, uniquement quand il est entièrement visible.
pub(crate) fn confirm_button_contains(area: Rect, x: u16, y: u16) -> bool {
    let popup = centered_rect(60, 30, area);
    let hints = confirmation_hints();
    let choices_width = 5 + hints.iter().map(|hint| hint.chars().count()).sum::<usize>();
    let inner_width = usize::from(popup.width.saturating_sub(2));
    if popup.height < 6 || choices_width > inner_width {
        return false;
    }
    let start = popup.x + 1 + ((inner_width - choices_width) / 2) as u16;
    let yes_width = 1 + hints[0].trim_end().chars().count() as u16;
    y == popup.y + 4 && x >= start && x < start + yes_width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_confirmation_distinguishes_control_characters_in_paths() {
        let path = "é\n\\n\u{1b}]52;c;text\u{7}\u{202e}.txt";
        let action = ConfirmAction::DiscardFile(path.to_string());
        let message = action.message();
        assert!(!message.chars().any(char::is_control));
        assert!(message.contains("\\x1b]52;c;text\\u{7}\\u{202e}"));
        assert_ne!(
            ConfirmAction::DiscardFile("a\nb".to_string()).message(),
            ConfirmAction::DiscardFile("a\\nb".to_string()).message()
        );
        assert_eq!(action, ConfirmAction::DiscardFile(path.to_string()));
    }
    use crate::i18n::{with_language, Language};

    #[test]
    fn test_confirm_title_is_localized_in_english() {
        with_language(Language::En, || {
            let title = ConfirmAction::BranchDelete("feature".to_string()).title();
            assert_eq!(title, "Confirm branch deletion");
        });
    }
}
