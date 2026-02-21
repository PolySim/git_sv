use crate::git::blame::FileBlame;
use crate::state::BlameState;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Widget pour afficher la vue blame d'un fichier.
pub struct BlameView<'a> {
    state: &'a BlameState,
}

impl<'a> BlameView<'a> {
    pub fn new(state: &'a BlameState) -> Self {
        Self { state }
    }
}

impl Widget for BlameView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout principal : titre + contenu
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Titre
        let title_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Blame: {} ", self.state.file_path))
            .style(Style::default().fg(Color::Cyan));
        title_block.render(chunks[0], buf);

        // Contenu du blame
        if let Some(ref blame) = self.state.blame {
            render_blame_content(blame, self.state, chunks[1], buf);
        } else {
            // Afficher un message de chargement
            let msg = Paragraph::new("Chargement du blame...")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL));
            msg.render(chunks[1], buf);
        }
    }
}

/// Affiche le contenu annoté du fichier.
fn render_blame_content(blame: &FileBlame, state: &BlameState, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Lignes annotées (↑↓: naviguer, Enter: aller au commit, Esc: fermer) ");

    let inner = block.inner(area);
    block.render(area, buf);

    // Calculer la hauteur visible
    let visible_height = inner.height as usize;

    // Ajuster le scroll si nécessaire
    let scroll_offset = state.scroll_offset;
    let selected_line = state.selected_line;

    // Récupérer les lignes visibles
    let start = scroll_offset;
    let end = (scroll_offset + visible_height).min(blame.lines.len());

    // Calculer la largeur maximale des colonnes
    let max_line_num_width = blame.lines.len().to_string().len().max(4);
    let hash_width = 7;
    let author_width = 15;

    // Rendu ligne par ligne
    for (i, blame_line) in blame.lines[start..end].iter().enumerate() {
        let y = inner.y + i as u16;
        let line_idx = start + i;

        // Style de la ligne (sélectionnée ou non)
        let is_selected = line_idx == selected_line;
        let bg_color = if is_selected {
            Color::DarkGray
        } else {
            Color::Reset
        };

        // Hash du commit (coloré)
        let hash_span = Span::styled(
            format!("{:width$} ", blame_line.short_hash, width = hash_width),
            Style::default().fg(Color::Yellow).bg(bg_color),
        );

        // Auteur (tronqué si nécessaire)
        let author = if blame_line.author.len() > author_width {
            format!("{}…", &blame_line.author[..author_width - 1])
        } else {
            format!("{:width$}", blame_line.author, width = author_width)
        };
        let author_span = Span::styled(
            format!("{} ", author),
            Style::default().fg(Color::Cyan).bg(bg_color),
        );

        // Numéro de ligne
        let line_num_span = Span::styled(
            format!(
                "{:width$} ",
                blame_line.line_num,
                width = max_line_num_width
            ),
            Style::default()
                .fg(Color::Gray)
                .bg(bg_color)
                .add_modifier(Modifier::DIM),
        );

        // Contenu de la ligne
        let content_span = Span::styled(
            &blame_line.content,
            Style::default().bg(bg_color),
        );

        // Construire la ligne complète
        let line = Line::from(vec![hash_span, author_span, line_num_span, content_span]);

        // Rendu de la ligne
        let line_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        let paragraph = Paragraph::new(line);
        paragraph.render(line_area, buf);
    }
}
