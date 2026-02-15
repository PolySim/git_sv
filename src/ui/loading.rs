//! Indicateur de chargement (spinner).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

/// Caractères du spinner (animation circulaire).
const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Intervalle entre chaque frame du spinner.
const SPINNER_INTERVAL: Duration = Duration::from_millis(80);

/// État du spinner de chargement.
#[derive(Debug)]
pub struct LoadingSpinner {
    /// Message à afficher pendant le chargement
    message: String,
    /// Instant de démarrage du spinner
    start_time: Instant,
    /// Dernier index d'animation calculé
    last_frame: usize,
}

impl LoadingSpinner {
    /// Crée un nouveau spinner avec un message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            start_time: Instant::now(),
            last_frame: 0,
        }
    }

    /// Met à jour et retourne l'index de frame actuel.
    pub fn current_frame(&mut self) -> usize {
        let elapsed = self.start_time.elapsed();
        let frame = (elapsed.as_millis() as usize / SPINNER_INTERVAL.as_millis() as usize)
            % SPINNER_CHARS.len();
        self.last_frame = frame;
        frame
    }

    /// Retourne le caractère du spinner actuel.
    pub fn current_char(&mut self) -> &'static str {
        let frame = self.current_frame();
        SPINNER_CHARS[frame]
    }

    /// Vérifie si le spinner est toujours actif (toujours vrai, utiliser pour timeout si besoin).
    pub fn is_active(&self) -> bool {
        true
    }

    /// Change le message du spinner.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

/// Rend un overlay de chargement avec spinner.
pub fn render_overlay(frame: &mut Frame, spinner: &mut LoadingSpinner, area: Rect) {
    use ratatui::widgets::Clear;

    // Zone centrale pour le spinner
    let popup_area = centered_rect(40, 20, area);

    // Effacer la zone
    frame.render_widget(Clear, popup_area);

    // Obtenir le caractère actuel
    let spinner_char = spinner.current_char();

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                spinner_char,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(spinner.message.clone(), Style::default().fg(Color::White)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Chargement ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}

/// Rend un spinner inline (pour les status bar).
pub fn render_inline(spinner: &mut LoadingSpinner) -> Line {
    let spinner_char = spinner.current_char();
    Line::from(vec![
        Span::styled(
            spinner_char,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(spinner.message.clone(), Style::default().fg(Color::White)),
    ])
}

/// Calcule un rectangle centré.
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
