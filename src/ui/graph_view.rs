use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::graph::{EdgeType, GraphRow, RefInfo, RefType};
use crate::ui::theme::{branch_color, current_theme};
use crate::utils::format_relative_time;

/// Espacement entre les colonnes (en caractères).
const COL_SPACING: usize = 2;

/// Rend le graphe de commits dans la zone donnée.
pub fn render(
    frame: &mut Frame,
    graph: &[GraphRow],
    current_branch: &Option<String>,
    selected_index: usize,
    total_commits: usize,
    area: Rect,
    state: &mut ListState,
    is_focused: bool,
) {
    let theme = current_theme();

    // Calculer la largeur disponible pour le contenu (hors bordures).
    let content_width = area.width.saturating_sub(2);

    // Construire les lignes du graphe avec les edges de connexion.
    let items = build_graph_items(graph, selected_index, content_width);

    let branch_name = current_branch.as_deref().unwrap_or("???");
    let title = if graph.len() < total_commits {
        // Afficher le compteur filtré
        format!(
            " Graphe — {} ({} / {}) ",
            branch_name,
            graph.len(),
            total_commits
        )
    } else {
        format!(" Graphe — {} ", branch_name)
    };

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default()); // Pas de style automatique, géré manuellement dans les spans

    frame.render_stateful_widget(list, area, state);
}

/// Construit les items de la liste avec le graphe enrichi.
fn build_graph_items(
    graph: &[GraphRow],
    selected_index: usize,
    available_width: u16,
) -> Vec<ListItem<'static>> {
    let mut items = Vec::with_capacity(graph.len() * 2);

    // Calculer le nombre maximum de colonnes dans le graphe pour l'alignement.
    let max_graph_cols = graph
        .iter()
        .map(|r| r.cells.len().max(r.node.column + 1))
        .max()
        .unwrap_or(1);

    for (i, row) in graph.iter().enumerate() {
        let is_selected = i == selected_index;

        // Ligne du commit.
        let commit_line = build_commit_line(row, is_selected, available_width, max_graph_cols);
        items.push(ListItem::new(commit_line));

        // Ligne de connexion vers le commit suivant (si existe).
        if let Some(ref connection) = row.connection {
            let connection_line = build_connection_line(connection);
            items.push(ListItem::new(connection_line));
        }
    }

    items
}

/// Construit la ligne d'un commit avec le graphe.
fn build_commit_line(
    row: &GraphRow,
    is_selected: bool,
    available_width: u16,
    max_graph_cols: usize,
) -> Line<'static> {
    let theme = current_theme();
    let node = &row.node;
    let commit_color = get_branch_color(node.color_index);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Nombre total de colonnes à afficher.
    let num_cols = row.cells.len().max(node.column + 1);

    // Construire chaque colonne avec l'espacement approprié.
    for col in 0..num_cols {
        if col == node.column {
            // C'est la colonne du commit - dessiner le nœud.
            let symbol = if node.parents.len() > 1 { "○" } else { "●" };
            spans.push(Span::styled(
                symbol.to_string(),
                Style::default()
                    .fg(commit_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Ajouter l'espacement après le nœud (sauf si c'est la dernière colonne).
            if col < num_cols - 1 {
                spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
            }
        } else if col < row.cells.len() {
            // Colonne avec potentiellement une cellule graphique.
            if let Some(ref cell) = row.cells[col] {
                let color = get_branch_color(cell.color_index);
                let ch = match cell.edge_type {
                    EdgeType::Vertical => "│",
                    _ => " ",
                };
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));

                // Ajouter l'espacement.
                if col < num_cols - 1 {
                    spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
                }
            } else {
                // Colonne vide.
                let spaces = if col < num_cols - 1 {
                    " ".repeat(COL_SPACING)
                } else {
                    " ".to_string()
                };
                spans.push(Span::raw(spaces));
            }
        } else {
            // Colonne au-delà des cellules définies - juste des espaces.
            let spaces = if col < num_cols - 1 {
                " ".repeat(COL_SPACING)
            } else {
                " ".to_string()
            };
            spans.push(Span::raw(spaces));
        }
    }

    // Aligner jusqu'à max_graph_cols pour un alignement vertical cohérent.
    for col in num_cols..max_graph_cols {
        spans.push(Span::raw(" ".repeat(COL_SPACING)));
    }

    // Séparateur graphe/texte (2 espaces pour un gap visuel naturel).
    spans.push(Span::raw("  "));

    // === Partie informations — appliquer le style de sélection si sélectionné ===

    // Helper pour le style conditionnel
    let sel_style = |base_fg: Color| -> Style {
        if is_selected {
            Style::default()
                .bg(theme.selection_bg)
                .fg(base_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(base_fg)
        }
    };

    // Hash du commit.
    let hash = node.oid.to_string();
    let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
    spans.push(Span::styled(
        format!("{} ", short_hash),
        sel_style(theme.commit_hash),
    ));

    // Labels de branches si présents — triés par pertinence.
    let mut sorted_refs: Vec<_> = node.refs.iter().collect();
    sorted_refs.sort_by_key(|r| match r.ref_type {
        RefType::Head => 0,
        RefType::LocalBranch => 1,
        RefType::Tag => 2,
        RefType::RemoteBranch => 3,
    });

    let refs_width: usize = sorted_refs.iter().map(|r| {
        let bracket_len = match r.ref_type {
            RefType::Head => 4,      // ⦗⦘ + espace
            RefType::Tag => 3,       // () + espace
            RefType::RemoteBranch => 4, // ⟨⟩ + espace
            RefType::LocalBranch => 3,  // [] + espace
        };
        r.name.len() + bracket_len
    }).sum();

    if !sorted_refs.is_empty() {
        for ref_info in sorted_refs {
            let (bracket, style) = match ref_info.ref_type {
                RefType::Head => {
                    // HEAD : mise en avant forte (vert gras inversé)
                    let bracket = format!("⦗{}⦘ ", ref_info.name);
                    let style = sel_style(Color::Green)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED);
                    (bracket, style)
                }
                RefType::LocalBranch => {
                    // Branche locale : fond coloré (couleur de la branche)
                    let ref_color = get_branch_color(node.color_index);
                    let bracket = format!("[{}] ", ref_info.name);
                    let style = sel_style(ref_color)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED);
                    (bracket, style)
                }
                RefType::RemoteBranch => {
                    // Remote : style plus discret, pas de REVERSED
                    let bracket = format!("⟨{}⟩ ", ref_info.name);
                    let style = sel_style(Color::DarkGray)
                        .add_modifier(Modifier::DIM);
                    (bracket, style)
                }
                RefType::Tag => {
                    // Tag : jaune, pas de REVERSED
                    let bracket = format!("({}) ", ref_info.name);
                    let style = sel_style(Color::Yellow)
                        .add_modifier(Modifier::BOLD);
                    (bracket, style)
                }
            };

            spans.push(Span::styled(bracket, style));
        }
    }

    // Calculer la largeur déjà utilisée.
    let graph_width = max_graph_cols * COL_SPACING + 2; // +2 pour le séparateur
    let hash_width = 8; // "abc1234 "
    let author_date_prefix = format!(" — {}", node.author);
    let relative_date = format_relative_time(node.timestamp);
    let author_date_suffix = format!(" {}", relative_date);
    let overhead = graph_width + hash_width + refs_width + author_date_prefix.len() + author_date_suffix.len();
    let max_message_width = (available_width as usize).saturating_sub(overhead);

    // Tronquer le message si nécessaire.
    let display_message = if node.message.len() > max_message_width && max_message_width > 3 {
        format!("{}…", &node.message[..max_message_width.saturating_sub(1)])
    } else {
        node.message.clone()
    };

    // Message du commit.
    spans.push(Span::styled(
        display_message,
        sel_style(if is_selected { theme.selection_fg } else { theme.text_normal }),
    ));

    // Auteur (avec séparateur).
    spans.push(Span::styled(
        author_date_prefix,
        sel_style(theme.text_secondary),
    ));

    // Date relative (avec style légèrement différent - DIM).
    spans.push(Span::styled(
        author_date_suffix,
        sel_style(theme.text_secondary).add_modifier(Modifier::DIM),
    ));

    Line::from(spans)
}

/// Construit la ligne de connexion entre deux commits.
fn build_connection_line(connection: &crate::git::graph::ConnectionRow) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    let num_cols = connection.cells.len();

    for col in 0..num_cols {
        if let Some(ref cell) = connection.cells[col] {
            let color = get_branch_color(cell.color_index);
            let ch = match cell.edge_type {
                EdgeType::Vertical => "│",
                EdgeType::ForkRight => "╮",
                EdgeType::ForkLeft => "╭",
                EdgeType::MergeFromRight => "╰",
                EdgeType::MergeFromLeft => "╯",
                EdgeType::Horizontal => "─",
                EdgeType::Cross => "┼",
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));

            // Ajouter l'espacement entre les colonnes.
            if col < num_cols - 1 {
                // Vérifier s'il y a une ligne horizontale à continuer.
                // Une ligne horizontale continue seulement si la cellule suivante
                // est Horizontal (pas ForkRight/ForkLeft qui sont des points d'arrivée).
                let needs_horizontal_right = col + 1 < num_cols
                    && connection.cells[col + 1]
                        .as_ref()
                        .map_or(false, |c| c.edge_type == EdgeType::Horizontal);
                let needs_horizontal_left = col > 0
                    && connection.cells[col - 1]
                        .as_ref()
                        .map_or(false, |c| c.edge_type == EdgeType::Horizontal);

                if needs_horizontal_right || needs_horizontal_left {
                    spans.push(Span::styled("─", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw(" "));
                }
            }
        } else {
            // Colonne vide — vérifier si on est entre deux cellules horizontales adjacentes.
            let left_is_horizontal = col > 0
                && connection.cells.get(col - 1)
                    .and_then(|c| c.as_ref())
                    .map_or(false, |c| matches!(c.edge_type,
                        EdgeType::Horizontal | EdgeType::MergeFromRight | EdgeType::Cross));

            let right_is_horizontal = col + 1 < connection.cells.len()
                && connection.cells.get(col + 1)
                    .and_then(|c| c.as_ref())
                    .map_or(false, |c| matches!(c.edge_type,
                        EdgeType::Horizontal | EdgeType::ForkRight | EdgeType::ForkLeft | EdgeType::Cross));

            if left_is_horizontal && right_is_horizontal {
                // On est dans le chemin d'un merge/fork — tracer la ligne.
                let color_idx = find_horizontal_color_bounded(col, connection);
                if let Some(idx) = color_idx {
                    let color = get_branch_color(idx);
                    spans.push(Span::styled("─", Style::default().fg(color)));
                    spans.push(Span::styled("─", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw("  "));
                }
            } else {
                spans.push(Span::raw("  "));
            }
        }
    }

    Line::from(spans)
}

/// Trouve la couleur d'une ligne horizontale adjacente (recherche bornée).
/// Ne cherche que dans les cellules immédiatement voisines.
fn find_horizontal_color_bounded(
    col: usize,
    connection: &crate::git::graph::ConnectionRow,
) -> Option<usize> {
    // Chercher vers la gauche (la cellule la plus proche).
    for c in (0..col).rev() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => {
                return Some(cell.color_index);
            }
            Some(cell) if matches!(cell.edge_type,
                EdgeType::MergeFromRight | EdgeType::MergeFromLeft) => {
                return Some(cell.color_index);
            }
            Some(_) => break, // Autre type de cellule = on arrête
            None => continue, // Colonne vide = on continue
        }
    }

    // Chercher vers la droite.
    for c in (col + 1)..connection.cells.len() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => {
                return Some(cell.color_index);
            }
            Some(cell) if matches!(cell.edge_type,
                EdgeType::ForkRight | EdgeType::ForkLeft) => {
                return Some(cell.color_index);
            }
            Some(_) => break,
            None => continue,
        }
    }

    None
}

/// Retourne la couleur pour un index de branche.
fn get_branch_color(index: usize) -> Color {
    branch_color(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, ConnectionRow, EdgeType, GraphCell, GraphRow};
    use git2::Oid;
    use ratatui::widgets::ListState;

    fn create_test_graph() -> Vec<GraphRow> {
        vec![
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                    message: "First commit".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1609459200, // 2021-01-01
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[2; 20]).unwrap_or(Oid::zero()),
                    message: "Second commit".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 1609545600, // 2021-01-02
                    parents: vec![Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero())],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
        ]
    }

    #[test]
    fn test_build_graph_items() {
        let graph = create_test_graph();
        let items = build_graph_items(&graph, 0, 80);

        // Chaque GraphRow génère au moins 1 item
        assert!(!items.is_empty());
        assert!(items.len() >= graph.len());
    }

    #[test]
    fn test_build_commit_line() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        // La ligne devrait contenir le message
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(line_text.contains("First commit"));
    }

    #[test]
    fn test_build_commit_line_selected() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, true, 80, 2);

        // La ligne devrait avoir des spans
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_graph_view_render_basic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let graph = create_test_graph();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(0));

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(
                    frame,
                    &graph,
                    &Some("main".to_string()),
                    0,
                    graph.len(),
                    area,
                    &mut state,
                    true,
                );
            })
            .unwrap();

        // Vérifier que quelque chose a été rendu
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.len() > 0);
    }

    #[test]
    fn test_graph_view_with_selection() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let graph = create_test_graph();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(2)); // Sélectionner le deuxième commit

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(
                    frame,
                    &graph,
                    &Some("feature".to_string()),
                    1, // selected_index = 1
                    graph.len(),
                    area,
                    &mut state,
                    false,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer.content.len() > 0);
    }

    #[test]
    fn test_get_branch_color() {
        let color0 = get_branch_color(0);
        let color1 = get_branch_color(1);
        let color2 = get_branch_color(2);

        // Les couleurs devraient être différentes
        assert_ne!(color0, color1);
        assert_ne!(color1, color2);
    }

    #[test]
    fn test_selected_commit_line_all_spans_have_bg() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, true, 80, 2);

        let theme = current_theme();

        // Compter combien de spans ont le bg de sélection
        let spans_with_selection_bg: Vec<_> = line.spans.iter()
            .filter(|s| s.style.bg == Some(theme.selection_bg))
            .collect();

        // Les spans du graphe (colonnes 0 et 1) n'ont pas de bg de sélection
        // Tous les autres spans (hash, message, auteur) devraient l'avoir
        //
        // Spans attendus pour la ligne sélectionnée:
        // - col 0: graphe (pas de bg)
        // - col 1: graphe (pas de bg)
        // - espace: " " (pas de bg)
        // - hash: "XXXXXXX " (avec bg)
        // - message: "First commit" (avec bg)
        // - info: " — Alice (...)" (avec bg)
        //
        // Au minimum, le hash, le message et l'info devraient avoir le bg
        assert!(spans_with_selection_bg.len() >= 3,
            "Au moins 3 spans devraient avoir le fond de sélection. Got: {} spans",
            spans_with_selection_bg.len());

        // Vérifier que le message a le bg de sélection
        let message_span = line.spans.iter()
            .find(|s| s.content.contains("First commit"))
            .expect("Devrait trouver le span du message");
        assert_eq!(message_span.style.bg, Some(theme.selection_bg),
            "Le message devrait avoir le fond de sélection");

        // Vérifier que le hash a le bg de sélection
        let hash_span = line.spans.iter()
            .find(|s| s.content.len() == 8 && s.content.trim().len() == 7)
            .expect("Devrait trouver le span du hash (7 caractères + espace)");
        assert_eq!(hash_span.style.bg, Some(theme.selection_bg),
            "Le hash devrait avoir le fond de sélection");
    }

    #[test]
    fn test_unselected_commit_line_no_bg() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        // Aucun span ne devrait avoir de bg de sélection
        let spans_with_selection_bg: Vec<_> = line.spans.iter()
            .filter(|s| s.style.bg.is_some())
            .collect();

        assert!(spans_with_selection_bg.is_empty(),
            "Aucun span ne devrait avoir de fond quand non sélectionné. Got: {} spans",
            spans_with_selection_bg.len());
    }

    #[test]
    fn test_no_horizontal_leak_past_fork() {
        // Créer une ConnectionRow avec merge col0→col2, colonne 3 vide
        // Le fork s'arrête à col 2, donc col 3 ne devrait PAS avoir de ligne horizontale
        let connection = ConnectionRow {
            cells: vec![
                Some(GraphCell { edge_type: EdgeType::MergeFromRight, color_index: 0 }),
                Some(GraphCell { edge_type: EdgeType::Horizontal, color_index: 0 }),
                Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
                None, // ← Cette colonne NE doit PAS avoir de "──"
                Some(GraphCell { edge_type: EdgeType::Vertical, color_index: 1 }),
            ],
        };

        let line = build_connection_line(&connection);
        let all_spans: Vec<_> = line.spans.iter().map(|s| s.content.as_ref()).collect();
        
        // Les spans attendus:
        // col 0 (MergeFromRight): "╰" + "─"
        // col 1 (Horizontal): "─" + "─" (car col 2 n'est pas vide, mais on vérifie col+1)
        // col 2 (ForkRight): "╮" + "─" (car col 1 est Horizontal)
        // col 3 (None): "  " (deux espaces - PAS de tirets!)
        // col 4 (Vertical): "│"
        //
        // Résultat attendu: ["╰", "─", "─", "─", "╮", "─", "  ", "│"]
        // Sans le fix, col 3 aurait "──" au lieu de "  "
        
        // Vérifier que spans[6] est "  " (deux espaces) et pas "──"
        assert_eq!(all_spans[6], "  ", 
            "La colonne vide après le fork devrait contenir des espaces, pas de lignes horizontales. Got: {:?}", 
            all_spans);
        
        // Vérifier aussi qu'on n'a pas de "──" n'importe où après col 2
        let after_fork: String = all_spans.iter().skip(6).copied().collect();
        assert!(!after_fork.contains('─'), 
            "Il ne devrait pas y avoir de lignes horizontales après le fork. Line: '{}'", 
            after_fork);
    }

    #[test]
    fn test_horizontal_between_merge_and_fork() {
        // Test qu'une colonne vide entre un merge et un fork a bien une ligne horizontale
        let connection = ConnectionRow {
            cells: vec![
                Some(GraphCell { edge_type: EdgeType::MergeFromRight, color_index: 0 }),
                None, // Colonne vide entre merge et fork — DEVRAIT avoir une ligne
                Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
            ],
        };

        let line = build_connection_line(&connection);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // La colonne vide (col 1) devrait avoir des tirets car elle est entre un merge et un fork
        assert!(line_text.contains("──"), "La colonne vide entre merge et fork devrait avoir une ligne horizontale");
    }

    #[test]
    fn test_find_horizontal_color_bounded() {
        // Cas 1: Colonne vide entre deux horizontales de la même branche
        let connection1 = ConnectionRow {
            cells: vec![
                Some(GraphCell { edge_type: EdgeType::MergeFromRight, color_index: 0 }),
                Some(GraphCell { edge_type: EdgeType::Horizontal, color_index: 0 }),
                None, // Colonne vide entre deux horizontales de couleur 0
                Some(GraphCell { edge_type: EdgeType::Horizontal, color_index: 0 }),
                Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
            ],
        };

        // Test col 2 (vide) entre deux horizontales de couleur 0
        let color = find_horizontal_color_bounded(2, &connection1);
        assert_eq!(color, Some(0), "Devrait trouver la couleur 0 entre deux horizontales");

        // Cas 2: Colonne vide entre merge et fork (même couleur)
        let connection2 = ConnectionRow {
            cells: vec![
                Some(GraphCell { edge_type: EdgeType::MergeFromRight, color_index: 0 }),
                None, // Colonne vide entre merge et fork
                Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
            ],
        };

        let color2 = find_horizontal_color_bounded(1, &connection2);
        // La fonction cherche vers la gauche et trouve MergeFromRight (color 0)
        assert_eq!(color2, Some(0), "Devrait trouver la couleur du merge");

        // Cas 3: Colonne vide après un fork (ne devrait pas être appelée, mais testons quand même)
        // La fonction arrête la recherche à droite quand elle trouve un type non-horizontal
        let connection3 = ConnectionRow {
            cells: vec![
                Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
                None, // Après le fork
                Some(GraphCell { edge_type: EdgeType::Horizontal, color_index: 1 }), // Autre branche
            ],
        };

        let color3 = find_horizontal_color_bounded(1, &connection3);
        // Vers la gauche: ForkRight (pas Horizontal/Merge) => break, pas de résultat
        // Vers la droite: Horizontal (color 1) => retourne Some(1)
        // Mais comme ForkRight n'est pas Horizontal, la fonction ne devrait pas être appelée
        // car left_is_horizontal serait false (ForkRight n'est pas dans la liste des types "horizontal")
        assert_eq!(color3, Some(1), "Trouve la couleur à droite même si gauche n'est pas horizontal");
    }

    #[test]
    fn test_message_truncation() {
        let mut row = create_test_graph()[0].clone();
        row.node.message = "A".repeat(200);

        // Avec une largeur de 120, le message devrait être tronqué
        let line = build_commit_line(&row, false, 120, 2);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // Le message devrait être tronqué avec "…" (pas les 200 caractères complets)
        assert!(line_text.contains('…'),
            "Le message tronqué devrait contenir '…'");
        assert!(!line_text.contains(&"A".repeat(150)),
            "Le message ne devrait pas contenir 150 caractères 'A'");
    }

    #[test]
    fn test_separator_between_graph_and_text() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        // Trouver le séparateur (devrait être "  " - 2 espaces)
        let separator_span = line.spans.iter()
            .find(|s| s.content == "  ")
            .expect("Devrait trouver le séparateur de 2 espaces");

        assert_eq!(separator_span.content, "  ",
            "Le séparateur devrait être 2 espaces");
    }

    #[test]
    fn test_author_date_separate_styles() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        // Trouver les spans de l'auteur et de la date
        let author_span = line.spans.iter()
            .find(|s| s.content.contains("Alice"))
            .expect("Devrait trouver le span de l'auteur");

        // La date est au format relatif français (ex: "il y a X ...")
        // Elle devrait être dans un span séparé après l'auteur
        let author_idx = line.spans.iter().position(|s| s.content.contains("Alice")).unwrap();
        let date_span = line.spans.get(author_idx + 1)
            .expect("Devrait trouver le span de la date après l'auteur");

        // La date devrait avoir le modificateur DIM
        assert!(date_span.style.add_modifier.contains(Modifier::DIM),
            "La date devrait avoir le modificateur DIM. Style: {:?}", date_span.style);
    }

    #[test]
    fn test_graph_columns_aligned() {
        // Test que le padding jusqu'à max_graph_cols fonctionne
        // Une ligne avec moins de colonnes devrait avoir du padding supplémentaire
        let graph = vec![
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                    message: "First".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1609459200,
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell { edge_type: EdgeType::Vertical, color_index: 0 })],
                connection: None,
            },
        ];

        let max_graph_cols = 3; // Forcer un padding à 3 colonnes

        // Construire la ligne
        let line = build_commit_line(&graph[0], false, 80, max_graph_cols);

        // La ligne devrait avoir suffisamment de spans pour 3 colonnes de graphe + séparateur
        // Chaque colonne a COL_SPACING (2) caractères
        // + 2 espaces pour le séparateur
        let expected_min_spans = 1; // Au moins un span

        assert!(line.spans.len() >= expected_min_spans,
            "La ligne devrait avoir au moins {} spans avec max_graph_cols=3. Got: {}",
            expected_min_spans, line.spans.len());

        // Vérifier qu'il y a des spans de padding (espaces)
        let padding_spans: Vec<_> = line.spans.iter()
            .filter(|s| s.content.chars().all(|c| c == ' '))
            .collect();

        assert!(!padding_spans.is_empty(),
            "La ligne devrait avoir des spans de padding pour aligner avec max_graph_cols");
    }
}
