use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::diff::{DiffLineType, DiffViewMode, FileDiff};

/// Largeur minimale pour le mode side-by-side (en caractères par colonne).
const MIN_SIDE_BY_SIDE_WIDTH: u16 = 60;

/// Rend le diff d'un fichier avec coloration syntaxique.
pub fn render(
    frame: &mut Frame,
    diff: Option<&FileDiff>,
    scroll_offset: usize,
    area: Rect,
    is_focused: bool,
    view_mode: DiffViewMode,
) {
    // Déterminer si on peut utiliser le mode side-by-side.
    let can_side_by_side = area.width >= MIN_SIDE_BY_SIDE_WIDTH * 2 + 3; // 2 colonnes + séparateur
    let effective_mode = if can_side_by_side {
        view_mode
    } else {
        DiffViewMode::Unified
    };

    match effective_mode {
        DiffViewMode::Unified => render_unified(frame, diff, scroll_offset, area, is_focused),
        DiffViewMode::SideBySide => {
            render_side_by_side(frame, diff, scroll_offset, area, is_focused)
        }
    }
}

/// Rend le diff en mode unifié.
fn render_unified(
    frame: &mut Frame,
    diff: Option<&FileDiff>,
    scroll_offset: usize,
    area: Rect,
    is_focused: bool,
) {
    let content = match diff {
        Some(d) => build_diff_lines(d),
        None => vec![Line::from("Sélectionnez un fichier pour voir le diff")],
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match diff {
        Some(d) => format!(" Diff — {} (+{}/-{}) ", d.path, d.additions, d.deletions),
        None => " Diff ".to_string(),
    };

    let paragraph = Paragraph::new(content)
        .scroll((scroll_offset as u16, 0))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        );

    frame.render_widget(paragraph, area);
}

/// Rend le diff en mode côte à côte.
fn render_side_by_side(
    frame: &mut Frame,
    diff: Option<&FileDiff>,
    scroll_offset: usize,
    area: Rect,
    is_focused: bool,
) {
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match diff {
        Some(d) => format!(
            " Diff (side-by-side) — {} (+{}/-{}) ",
            d.path, d.additions, d.deletions
        ),
        None => " Diff (side-by-side) ".to_string(),
    };

    // Diviser l'aire en deux colonnes avec un séparateur au milieu.
    let chunks = Layout::default()
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
    if let Some(d) = diff {
        let (left_lines, right_lines) = build_side_by_side_lines(d);

        // Colonne ancienne (suppressions + contexte).
        let left_paragraph = Paragraph::new(left_lines).scroll((scroll_offset as u16, 0));
        frame.render_widget(left_paragraph, inner_chunks[0]);

        // Séparateur vertical.
        let separator =
            Paragraph::new(vec![Line::from("│")]).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(separator, inner_chunks[1]);

        // Colonne nouvelle (ajouts + contexte).
        let right_paragraph = Paragraph::new(right_lines).scroll((scroll_offset as u16, 0));
        frame.render_widget(right_paragraph, inner_chunks[2]);
    } else {
        let msg = vec![Line::from("Sélectionnez un fichier pour voir le diff")];
        let paragraph = Paragraph::new(msg);
        frame.render_widget(paragraph, inner_chunks[0]);
    }
}

/// Construit les lignes de diff avec coloration (mode unifié).
fn build_diff_lines(diff: &FileDiff) -> Vec<Line<'static>> {
    diff.lines
        .iter()
        .map(|line| {
            let (prefix, fg_color, bg_color) = match line.line_type {
                DiffLineType::Addition => ("+", Color::Green, Some(Color::Rgb(0, 40, 0))),
                DiffLineType::Deletion => ("-", Color::Red, Some(Color::Rgb(40, 0, 0))),
                DiffLineType::Context => (" ", Color::Reset, None),
                DiffLineType::HunkHeader => ("", Color::Cyan, None),
            };

            let mut spans = Vec::new();

            if line.line_type == DiffLineType::HunkHeader {
                // Header de hunk : pas de numéros de ligne, juste le contenu en cyan.
                spans.push(Span::styled(
                    line.content.clone(),
                    Style::default()
                        .fg(Color::Cyan)
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
                    Style::default().fg(Color::DarkGray),
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
    let pairs = align_diff_lines(&diff.lines);

    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for pair in pairs {
        // Ligne gauche (ancien).
        let left_line = match &pair.left {
            Some(line) => {
                let (fg_color, bg_color) = match line.line_type {
                    DiffLineType::Deletion => (Color::Red, Some(Color::Rgb(40, 0, 0))),
                    DiffLineType::Context => (Color::Reset, None),
                    _ => (Color::Reset, None),
                };

                let lineno = line
                    .old_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());

                let mut spans = Vec::new();
                spans.push(Span::styled(
                    format!("{} ", lineno),
                    Style::default().fg(Color::DarkGray),
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
                Line::from(Span::styled("     ", Style::default().fg(Color::DarkGray)))
            }
        };
        left_lines.push(left_line);

        // Ligne droite (nouveau).
        let right_line = match &pair.right {
            Some(line) => {
                let (fg_color, bg_color) = match line.line_type {
                    DiffLineType::Addition => (Color::Green, Some(Color::Rgb(0, 40, 0))),
                    DiffLineType::Context => (Color::Reset, None),
                    _ => (Color::Reset, None),
                };

                let lineno = line
                    .new_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());

                let mut spans = Vec::new();
                spans.push(Span::styled(
                    format!("{} ", lineno),
                    Style::default().fg(Color::DarkGray),
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
                Line::from(Span::styled("     ", Style::default().fg(Color::DarkGray)))
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
    use crate::git::diff::DiffLine;

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
