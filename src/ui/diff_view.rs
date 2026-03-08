//! Rendu des diffs de fichiers (mode unifié et side-by-side).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::diff::{DiffLineType, DiffViewMode, FileDiff};
use crate::i18n::{text, text_owned};
use crate::ui::theme::{current_theme, Theme};

/// Largeur minimale pour le mode side-by-side (en caractères par colonne).
const MIN_SIDE_BY_SIDE_WIDTH: u16 = 60;

/// Contexte de rendu du panneau de diff.
pub struct DiffRenderContext<'a> {
    pub diff: Option<&'a FileDiff>,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
    pub area: Rect,
    pub is_focused: bool,
    pub view_mode: DiffViewMode,
    pub is_fullscreen: bool,
}

struct DiffPanelContext<'a> {
    diff: Option<&'a FileDiff>,
    scroll_offset: usize,
    horizontal_offset: usize,
    area: Rect,
    is_focused: bool,
    is_fullscreen: bool,
    theme: &'a Theme,
}

/// Rend le diff d'un fichier avec coloration syntaxique.
pub fn render(frame: &mut Frame, ctx: DiffRenderContext<'_>) -> usize {
    let DiffRenderContext {
        diff,
        scroll_offset,
        horizontal_offset,
        area,
        is_focused,
        view_mode,
        is_fullscreen,
    } = ctx;

    let theme = current_theme();
    // Déterminer si on peut utiliser le mode side-by-side.
    let can_side_by_side = area.width >= MIN_SIDE_BY_SIDE_WIDTH * 2 + 3; // 2 colonnes + séparateur
    let effective_mode = if can_side_by_side {
        view_mode
    } else {
        DiffViewMode::Unified
    };

    let panel_ctx = DiffPanelContext {
        diff,
        scroll_offset,
        horizontal_offset,
        area,
        is_focused,
        is_fullscreen,
        theme,
    };

    match effective_mode {
        DiffViewMode::Unified => render_unified(frame, panel_ctx),
        DiffViewMode::SideBySide => render_side_by_side(frame, panel_ctx),
    }
}

/// Rend le diff en mode unifié.
fn render_unified(frame: &mut Frame, ctx: DiffPanelContext<'_>) -> usize {
    let DiffPanelContext {
        diff,
        scroll_offset,
        horizontal_offset,
        area,
        is_focused,
        is_fullscreen,
        theme,
    } = ctx;

    let content = match diff {
        Some(d) => build_diff_lines(d),
        None => vec![Line::from(text(
            "Selectionnez un fichier pour voir le diff",
            "Select a file to view the diff",
        ))],
    };

    let total_lines = content.len();
    let visible_height = area.height.saturating_sub(2) as usize; // -2 pour les bordures
    let max_scroll = total_lines.saturating_sub(visible_height);

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let title = match diff {
        Some(d) => {
            let fullscreen_indicator = if is_fullscreen { " [ZOOM]" } else { "" };
            let scroll_indicator = if total_lines > 0 {
                format!(" [{}/{}]", scroll_offset.min(max_scroll) + 1, total_lines)
            } else {
                String::new()
            };
            text_owned(
                format!(
                    " Diff - {} (+{}/-{}){}{} ",
                    d.path, d.additions, d.deletions, fullscreen_indicator, scroll_indicator
                ),
                format!(
                    " Diff - {} (+{}/-{}){}{} ",
                    d.path, d.additions, d.deletions, fullscreen_indicator, scroll_indicator
                ),
            )
        }
        None => {
            let fullscreen_indicator = if is_fullscreen { " [ZOOM]" } else { "" };
            text_owned(
                format!(" Diff{} ", fullscreen_indicator),
                format!(" Diff{} ", fullscreen_indicator),
            )
        }
    };

    let title = if is_focused {
        format!(">{}<", title)
    } else {
        title
    };

    let paragraph = Paragraph::new(content)
        .scroll((scroll_offset as u16, horizontal_offset as u16))
        .style(Style::default().fg(theme.text_normal).bg(theme.background))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        );

    frame.render_widget(paragraph, area);
    total_lines
}

/// Rend le diff en mode côte à côte.
fn render_side_by_side(frame: &mut Frame, ctx: DiffPanelContext<'_>) -> usize {
    let DiffPanelContext {
        diff,
        scroll_offset,
        horizontal_offset,
        area,
        is_focused,
        is_fullscreen,
        theme,
    } = ctx;

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let title = match diff {
        Some(d) => {
            let fullscreen_indicator = if is_fullscreen { " [ZOOM]" } else { "" };
            text_owned(
                format!(
                    " Diff (cote a cote) - {} (+{}/-{}){} ",
                    d.path, d.additions, d.deletions, fullscreen_indicator
                ),
                format!(
                    " Diff (side-by-side) - {} (+{}/-{}){} ",
                    d.path, d.additions, d.deletions, fullscreen_indicator
                ),
            )
        }
        None => {
            let fullscreen_indicator = if is_fullscreen { " [ZOOM]" } else { "" };
            text_owned(
                format!(" Diff (cote a cote){} ", fullscreen_indicator),
                format!(" Diff (side-by-side){} ", fullscreen_indicator),
            )
        }
    };

    let title = if is_focused {
        format!(">{}<", title)
    } else {
        title
    };

    // Diviser l'aire en deux colonnes avec un séparateur au milieu.
    let _chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(1), // Séparateur
            Constraint::Percentage(50),
        ])
        .margin(0)
        .split(area);

    // Rendre le bloc principal avec bordure.
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // La bordure prend toute l'aire, mais on rend le contenu à l'intérieur.
    frame.render_widget(block, area);

    // Ajuster les chunks pour tenir compte des bordures (marge de 1 de chaque côté).
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
        .split(inner_area);

    // Rendre le contenu si disponible.
    let total_lines = if let Some(d) = diff {
        let (left_lines, right_lines) = build_side_by_side_lines(d);
        let total = left_lines.len().max(right_lines.len());

        // Colonne ancienne (suppressions + contexte).
        let left_paragraph = Paragraph::new(left_lines)
            .style(Style::default().fg(theme.text_normal).bg(theme.background))
            .scroll((scroll_offset as u16, horizontal_offset as u16));
        frame.render_widget(left_paragraph, inner_chunks[0]);

        // Séparateur vertical.
        let separator = Paragraph::new(vec![Line::from("│")]).style(
            Style::default()
                .fg(theme.border_inactive)
                .bg(theme.background),
        );
        frame.render_widget(separator, inner_chunks[1]);

        // Colonne nouvelle (ajouts + contexte).
        let right_paragraph = Paragraph::new(right_lines)
            .style(Style::default().fg(theme.text_normal).bg(theme.background))
            .scroll((scroll_offset as u16, horizontal_offset as u16));
        frame.render_widget(right_paragraph, inner_chunks[2]);

        total
    } else {
        let msg = vec![Line::from(text(
            "Selectionnez un fichier pour voir le diff",
            "Select a file to view the diff",
        ))];
        let paragraph = Paragraph::new(msg);
        frame.render_widget(paragraph, inner_chunks[0]);
        0
    };

    total_lines
}

/// Construit les lignes de diff avec coloration (mode unifié).
fn build_diff_lines(diff: &FileDiff) -> Vec<Line<'static>> {
    let theme = current_theme();
    diff.lines
        .iter()
        .map(|line| {
            let (prefix, fg_color, bg_color) = match line.line_type {
                DiffLineType::Addition => ("+", theme.success, Some(theme.ours_bg)),
                DiffLineType::Deletion => ("-", theme.error, Some(theme.theirs_bg)),
                DiffLineType::Context => (" ", theme.text_normal, None),
                DiffLineType::HunkHeader => ("", theme.info, None),
            };

            let mut spans = Vec::new();

            if line.line_type == DiffLineType::HunkHeader {
                // Header de hunk : pas de numéros de ligne, juste le contenu en cyan.
                spans.push(Span::styled(
                    line.content.clone(),
                    Style::default()
                        .fg(theme.info)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
            } else {
                // Numéros de lignes.
                let old_no = line
                    .old_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                let new_no = line
                    .new_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                spans.push(Span::styled(
                    format!("{} {} ", old_no, new_no),
                    Style::default().fg(theme.text_secondary),
                ));

                // Préfixe et contenu avec coloration.
                let style = Style::default().fg(fg_color);
                let style = if let Some(bg) = bg_color {
                    style.bg(bg)
                } else {
                    style
                };
                spans.push(Span::styled(format!("{}{}", prefix, line.content), style));
            }

            Line::from(spans)
        })
        .collect()
}

/// Une ligne pour l'affichage side-by-side.
#[derive(Debug, Clone)]
struct SideBySideLine {
    /// Numéro de ligne dans l'ancien fichier (si applicable).
    pub old_lineno: Option<u32>,
    /// Numéro de ligne dans le nouveau fichier (si applicable).
    pub new_lineno: Option<u32>,
    /// Contenu textuel de la ligne.
    pub content: String,
    /// Type de la ligne.
    pub line_type: DiffLineType,
}

/// Construit les paires de lignes pour l'affichage side-by-side.
///
/// Retourne (lignes_gauche, lignes_droite) synchronisées.
fn build_side_by_side_lines(diff: &FileDiff) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let theme = current_theme();
    let pairs = align_diff_lines(&diff.lines);

    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for pair in pairs {
        // Ligne gauche (ancien).
        let left_line = match &pair.left {
            Some(line) => {
                let (fg_color, bg_color) = match line.line_type {
                    DiffLineType::Deletion => (theme.error, Some(theme.theirs_bg)),
                    DiffLineType::Context => (theme.text_normal, None),
                    _ => (theme.text_normal, None),
                };

                let lineno = line
                    .old_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());

                let mut spans = Vec::new();
                spans.push(Span::styled(
                    format!("{} ", lineno),
                    Style::default().fg(theme.text_secondary),
                ));

                let style = Style::default().fg(fg_color);
                let style = if let Some(bg) = bg_color {
                    style.bg(bg)
                } else {
                    style
                };
                spans.push(Span::styled(format!("- {}", line.content), style));

                Line::from(spans)
            }
            None => {
                // Placeholder pour alignement.
                Line::from(Span::styled(
                    "     ",
                    Style::default().fg(theme.text_secondary),
                ))
            }
        };
        left_lines.push(left_line);

        // Ligne droite (nouveau).
        let right_line = match &pair.right {
            Some(line) => {
                let (fg_color, bg_color) = match line.line_type {
                    DiffLineType::Addition => (theme.success, Some(theme.ours_bg)),
                    DiffLineType::Context => (theme.text_normal, None),
                    _ => (theme.text_normal, None),
                };

                let lineno = line
                    .new_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());

                let mut spans = Vec::new();
                spans.push(Span::styled(
                    format!("{} ", lineno),
                    Style::default().fg(theme.text_secondary),
                ));

                let style = Style::default().fg(fg_color);
                let style = if let Some(bg) = bg_color {
                    style.bg(bg)
                } else {
                    style
                };
                spans.push(Span::styled(format!("+ {}", line.content), style));

                Line::from(spans)
            }
            None => {
                // Placeholder pour alignement.
                Line::from(Span::styled(
                    "     ",
                    Style::default().fg(theme.text_secondary),
                ))
            }
        };
        right_lines.push(right_line);
    }

    (left_lines, right_lines)
}

/// Une paire de lignes alignées pour le rendu side-by-side.
#[derive(Debug, Clone)]
struct LinePair {
    /// Ligne de l'ancien fichier (suppression ou contexte).
    pub left: Option<SideBySideLine>,
    /// Ligne du nouveau fichier (ajout ou contexte).
    pub right: Option<SideBySideLine>,
}

/// Aligne les lignes du diff en paires pour l'affichage side-by-side.
///
/// Cette fonction normalise les hunks en paires (left, right) où :
/// - Les lignes de contexte apparaissent des deux côtés
/// - Les suppressions ont un placeholder à droite
/// - Les ajouts ont un placeholder à gauche
fn align_diff_lines(lines: &[crate::git::diff::DiffLine]) -> Vec<LinePair> {
    let mut pairs = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];

        match line.line_type {
            DiffLineType::HunkHeader => {
                // Les headers de hunk sont affichés centrés (sur les deux colonnes).
                pairs.push(LinePair {
                    left: Some(SideBySideLine {
                        old_lineno: None,
                        new_lineno: None,
                        content: line.content.clone(),
                        line_type: DiffLineType::HunkHeader,
                    }),
                    right: None,
                });
                i += 1;
            }
            DiffLineType::Context => {
                // Ligne de contexte : apparait des deux côtés.
                pairs.push(LinePair {
                    left: Some(SideBySideLine {
                        old_lineno: line.old_lineno,
                        new_lineno: None,
                        content: line.content.clone(),
                        line_type: DiffLineType::Context,
                    }),
                    right: Some(SideBySideLine {
                        old_lineno: None,
                        new_lineno: line.new_lineno,
                        content: line.content.clone(),
                        line_type: DiffLineType::Context,
                    }),
                });
                i += 1;
            }
            DiffLineType::Deletion => {
                // Regrouper les suppressions consécutives.
                let mut deletions = Vec::new();
                while i < lines.len() && lines[i].line_type == DiffLineType::Deletion {
                    deletions.push(SideBySideLine {
                        old_lineno: lines[i].old_lineno,
                        new_lineno: None,
                        content: lines[i].content.clone(),
                        line_type: DiffLineType::Deletion,
                    });
                    i += 1;
                }

                // Regrouper les ajouts consécutifs qui suivent (s'ils existent).
                let mut additions = Vec::new();
                while i < lines.len() && lines[i].line_type == DiffLineType::Addition {
                    additions.push(SideBySideLine {
                        old_lineno: None,
                        new_lineno: lines[i].new_lineno,
                        content: lines[i].content.clone(),
                        line_type: DiffLineType::Addition,
                    });
                    i += 1;
                }

                // Aligner les suppressions et ajouts.
                let max_len = deletions.len().max(additions.len());
                for idx in 0..max_len {
                    let left = deletions.get(idx).cloned();
                    let right = additions.get(idx).cloned();
                    pairs.push(LinePair { left, right });
                }
            }
            DiffLineType::Addition => {
                // Ajout sans suppression précédente : placeholder à gauche.
                pairs.push(LinePair {
                    left: None,
                    right: Some(SideBySideLine {
                        old_lineno: None,
                        new_lineno: line.new_lineno,
                        content: line.content.clone(),
                        line_type: DiffLineType::Addition,
                    }),
                });
                i += 1;
            }
        }
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::{DiffLine, DiffLineType};

    fn create_test_line(
        line_type: DiffLineType,
        content: &str,
        old_no: Option<u32>,
        new_no: Option<u32>,
    ) -> DiffLine {
        DiffLine {
            line_type,
            content: content.to_string(),
            old_lineno: old_no,
            new_lineno: new_no,
        }
    }

    #[test]
    fn test_align_diff_lines_context_only() {
        let lines = vec![
            create_test_line(DiffLineType::Context, "line 1", Some(1), Some(1)),
            create_test_line(DiffLineType::Context, "line 2", Some(2), Some(2)),
        ];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 2);

        // Chaque ligne de contexte doit avoir left et right.
        assert!(pairs[0].left.is_some());
        assert!(pairs[0].right.is_some());
        assert_eq!(pairs[0].left.as_ref().unwrap().content, "line 1");
        assert_eq!(pairs[0].right.as_ref().unwrap().content, "line 1");
    }

    #[test]
    fn test_align_diff_lines_with_deletion() {
        let lines = vec![create_test_line(
            DiffLineType::Deletion,
            "old line",
            Some(5),
            None,
        )];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 1);

        assert!(pairs[0].left.is_some());
        assert!(pairs[0].right.is_none());
        assert_eq!(pairs[0].left.as_ref().unwrap().content, "old line");
    }

    #[test]
    fn test_align_diff_lines_with_addition() {
        let lines = vec![create_test_line(
            DiffLineType::Addition,
            "new line",
            None,
            Some(5),
        )];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 1);

        assert!(pairs[0].left.is_none());
        assert!(pairs[0].right.is_some());
        assert_eq!(pairs[0].right.as_ref().unwrap().content, "new line");
    }

    #[test]
    fn test_align_diff_lines_simple_change() {
        // Un remplacement simple : suppression suivie d'ajout.
        let lines = vec![
            create_test_line(DiffLineType::Deletion, "old content", Some(10), None),
            create_test_line(DiffLineType::Addition, "new content", None, Some(10)),
        ];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 1);

        // Doivent être alignés sur la même ligne.
        assert!(pairs[0].left.is_some());
        assert!(pairs[0].right.is_some());
        assert_eq!(pairs[0].left.as_ref().unwrap().content, "old content");
        assert_eq!(pairs[0].right.as_ref().unwrap().content, "new content");
    }

    #[test]
    fn test_align_diff_lines_hunk_header() {
        let lines = vec![
            create_test_line(DiffLineType::HunkHeader, "@@ -10,5 +10,7 @@", None, None),
            create_test_line(DiffLineType::Context, "context line", Some(10), Some(10)),
        ];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 2);

        // Le header doit avoir left mais pas right.
        assert!(pairs[0].left.is_some());
        assert!(pairs[0].right.is_none());
        assert_eq!(
            pairs[0].left.as_ref().unwrap().line_type,
            DiffLineType::HunkHeader
        );
    }

    #[test]
    fn test_align_diff_lines_multiple_changes() {
        // Deux suppressions suivies de trois ajouts.
        let lines = vec![
            create_test_line(DiffLineType::Deletion, "old 1", Some(1), None),
            create_test_line(DiffLineType::Deletion, "old 2", Some(2), None),
            create_test_line(DiffLineType::Addition, "new A", None, Some(1)),
            create_test_line(DiffLineType::Addition, "new B", None, Some(2)),
            create_test_line(DiffLineType::Addition, "new C", None, Some(3)),
        ];

        let pairs = align_diff_lines(&lines);
        assert_eq!(pairs.len(), 3); // max(2, 3) = 3 paires

        // Paire 1 : old 1 + new A.
        assert!(pairs[0].left.is_some());
        assert!(pairs[0].right.is_some());

        // Paire 2 : old 2 + new B.
        assert!(pairs[1].left.is_some());
        assert!(pairs[1].right.is_some());

        // Paire 3 : placeholder + new C.
        assert!(pairs[2].left.is_none());
        assert!(pairs[2].right.is_some());
    }
}
