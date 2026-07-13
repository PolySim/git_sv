use crate::error::Result;
use crate::state::ViewMode;
use crate::utils::flash_success;

use super::super::traits::HandlerContext;

/// Gère la copie dans le presse-papier.
pub(super) fn handle_copy_to_clipboard(ctx: &mut HandlerContext) -> Result<()> {
    use crate::state::{BranchesSection, FocusPanel, ProjectTreeFocus, StagingFocus};

    let mut text_to_copy = String::new();

    match ctx.state.view_mode {
        ViewMode::Graph => {
            // Graph view: copier hash + message du commit sélectionné
            if let Some(commit) = ctx.state.selected_commit() {
                let oid_str = commit.oid.to_string();
                let message = commit.message.lines().next().unwrap_or("");
                text_to_copy = format!("{} {}", oid_str, message);
            } else {
                return Ok(());
            }

            // Ajouter le contenu du panneau BottomRight si focus est sur BottomLeft ou BottomRight
            match ctx.state.focus {
                FocusPanel::BottomLeft => {
                    if let Some(file) = ctx
                        .state
                        .graph_view
                        .commit_files
                        .get(ctx.state.graph_view.file_selected_index)
                    {
                        text_to_copy = file.path.clone();
                        if let Some(ref diff) = ctx.state.graph_view.selected_file_diff {
                            let diff_text = diff
                                .lines
                                .iter()
                                .map(|line| line.content.trim_end_matches('\n').to_string())
                                .collect::<Vec<_>>()
                                .join("\n");
                            text_to_copy = format!("{}\n\n{}", text_to_copy, diff_text);
                        }
                    }
                }
                FocusPanel::BottomRight => {
                    if let Some(ref diff) = ctx.state.graph_view.selected_file_diff {
                        text_to_copy = diff
                            .lines
                            .iter()
                            .map(|line| line.content.trim_end_matches('\n').to_string())
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
                _ => {}
            }
        }
        ViewMode::Staging => match ctx.state.staging_state.focus {
            StagingFocus::Unstaged => {
                text_to_copy = ctx
                    .state
                    .staging_state
                    .unstaged_files()
                    .get(ctx.state.staging_state.unstaged_selected())
                    .map(|f| f.path.clone())
                    .unwrap_or_default();
            }
            StagingFocus::Staged => {
                text_to_copy = ctx
                    .state
                    .staging_state
                    .staged_files()
                    .get(ctx.state.staging_state.staged_selected())
                    .map(|f| f.path.clone())
                    .unwrap_or_default();
            }
            StagingFocus::Diff => {
                text_to_copy = ctx
                    .state
                    .staging_state
                    .current_diff
                    .as_ref()
                    .map(|diff| {
                        diff.lines
                            .iter()
                            .map(|line| line.content.trim_end_matches('\n').to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
            }
            StagingFocus::CommitMessage => {
                text_to_copy = ctx.state.staging_state.commit_message.clone();
            }
        },
        ViewMode::Branches => match ctx.state.branches_view_state.section {
            BranchesSection::Branches => {
                text_to_copy = ctx
                    .state
                    .branches_view_state
                    .selected_branch()
                    .map(|b| b.name.clone())
                    .unwrap_or_default();
            }
            BranchesSection::Worktrees => {
                text_to_copy = ctx
                    .state
                    .branches_view_state
                    .worktrees
                    .selected_item()
                    .map(|w| format!("{}: {}", w.name, w.path))
                    .unwrap_or_default();
            }
            BranchesSection::Stashes => {
                text_to_copy = ctx
                    .state
                    .branches_view_state
                    .stashes
                    .selected_item()
                    .map(|s| {
                        format!(
                            "{}: {}",
                            s.oid.to_string().get(0..7).unwrap_or(""),
                            s.message
                        )
                    })
                    .unwrap_or_default();
            }
        },
        ViewMode::ProjectTree => match ctx.state.project_tree_state.focus {
            ProjectTreeFocus::Tree => {
                text_to_copy = ctx
                    .state
                    .project_tree_state
                    .selected_entry()
                    .map(|entry| entry.path.clone())
                    .unwrap_or_default();
            }
            ProjectTreeFocus::History => {
                text_to_copy = ctx
                    .state
                    .project_tree_state
                    .history
                    .selected_item()
                    .map(|commit| format!("{} {}", commit.oid, commit.message))
                    .unwrap_or_default();
            }
            ProjectTreeFocus::ChangedFiles => {
                let oid = ctx
                    .state
                    .project_tree_state
                    .selected_history_commit()
                    .map(|commit| commit.oid);
                let path = ctx
                    .state
                    .project_tree_state
                    .selected_changed_file()
                    .map(|file| file.path.clone());
                let (Some(oid), Some(path)) = (oid, path) else {
                    return Ok(());
                };
                match ctx.state.repo.file_content_at_commit(oid, &path) {
                    Ok(Some(content)) => text_to_copy = content,
                    Ok(None) => {
                        ctx.state.set_flash_message(format!(
                            "Le fichier '{}' n'existe pas dans ce commit",
                            path
                        ));
                        return Ok(());
                    }
                    Err(error) => {
                        ctx.state.set_flash_message(crate::utils::flash_error(
                            "copie du fichier au commit",
                            error,
                        ));
                        return Ok(());
                    }
                }
            }
            ProjectTreeFocus::Diff => {
                text_to_copy = ctx
                    .state
                    .project_tree_state
                    .selected_diff
                    .as_ref()
                    .map(|diff| {
                        diff.lines
                            .iter()
                            .map(|line| {
                                let prefix = match line.line_type {
                                    crate::git::diff::DiffLineType::Addition => "+",
                                    crate::git::diff::DiffLineType::Deletion => "-",
                                    crate::git::diff::DiffLineType::Context => " ",
                                    crate::git::diff::DiffLineType::HunkHeader => "",
                                };
                                format!("{}{}", prefix, line.content)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
            }
        },
        ViewMode::Conflicts => {
            if let Some(ref conflicts_state) = ctx.state.conflicts_state {
                if let Some(file) = conflicts_state.all_files.get(conflicts_state.file_selected) {
                    text_to_copy = file.path.clone();
                }
            }
        }
        ViewMode::Blame => {
            if let Some(ref blame_state) = ctx.state.blame_state {
                if let Some(ref blame) = blame_state.blame {
                    text_to_copy = blame
                        .lines
                        .iter()
                        .map(|l| l.content.clone())
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
        }
        ViewMode::Help => {
            // Pas de contenu à copier en mode aide
        }
    }

    // Copier dans le clipboard
    if !text_to_copy.is_empty() {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
        clipboard
            .set_text(&text_to_copy)
            .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
        ctx.state
            .set_flash_message(flash_success("Copié dans le presse-papier"));
    }

    Ok(())
}
