use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::git::branch::BranchInfo;
use crate::git::diff::DiffFile;
use crate::git::graph::GraphRow;
use crate::git::repo::{GitRepo, StatusEntry};
use crate::ui;

/// Nombre maximum de commits à charger.
const MAX_COMMITS: usize = 200;

/// Actions possibles déclenchées par l'utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    GoTop,
    GoBottom,
    Select,
    CommitPrompt,
    StashPrompt,
    MergePrompt,
    BranchList,
    ToggleHelp,
    Refresh,
    SwitchBottomMode,
    BranchCheckout,
    BranchCreate,
    BranchDelete,
    CloseBranchPanel,
}

/// Mode d'affichage actif.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Graph,
    Help,
}

/// Mode du panneau bas-gauche.
#[derive(Debug, Clone, PartialEq)]
pub enum BottomLeftMode {
    CommitFiles,
    WorkingDir,
}

/// Panneau actuellement focalisé.
#[derive(Debug, Clone, PartialEq)]
pub enum FocusPanel {
    Graph,
    Files,
    Detail,
}

/// État principal de l'application.
pub struct App {
    pub repo: GitRepo,
    pub repo_path: String,
    pub graph: Vec<GraphRow>,
    pub status_entries: Vec<StatusEntry>,
    pub commit_files: Vec<DiffFile>,
    pub branches: Vec<BranchInfo>,
    pub current_branch: Option<String>,
    pub selected_index: usize,
    pub graph_state: ListState,
    pub view_mode: ViewMode,
    pub bottom_left_mode: BottomLeftMode,
    pub focus: FocusPanel,
    pub show_branch_panel: bool,
    pub branch_selected: usize,
    pub flash_message: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    /// Crée une nouvelle instance de l'application.
    pub fn new(repo: GitRepo, repo_path: String) -> Result<Self> {
        let mut graph_state = ListState::default();
        graph_state.select(Some(0));

        let mut app = Self {
            repo,
            repo_path,
            graph: Vec::new(),
            status_entries: Vec::new(),
            commit_files: Vec::new(),
            branches: Vec::new(),
            current_branch: None,
            selected_index: 0,
            graph_state,
            view_mode: ViewMode::Graph,
            bottom_left_mode: BottomLeftMode::CommitFiles,
            focus: FocusPanel::Graph,
            show_branch_panel: false,
            branch_selected: 0,
            flash_message: None,
            should_quit: false,
        };
        app.refresh()?;
        Ok(app)
    }

    /// Rafraîchit les données depuis le repository git.
    pub fn refresh(&mut self) -> Result<()> {
        self.current_branch = self.repo.current_branch().ok();
        self.graph = self.repo.build_graph(MAX_COMMITS).unwrap_or_default();
        self.status_entries = self.repo.status().unwrap_or_default();

        // Réajuster la sélection si nécessaire.
        if self.selected_index >= self.graph.len() && !self.graph.is_empty() {
            self.selected_index = self.graph.len() - 1;
        }

        // Charger les fichiers du commit sélectionné.
        self.update_commit_files();

        Ok(())
    }

    /// Met à jour la liste des fichiers du commit sélectionné.
    fn update_commit_files(&mut self) {
        if let Some(row) = self.graph.get(self.selected_index) {
            self.commit_files = self.repo.commit_diff(row.node.oid).unwrap_or_default();
        } else {
            self.commit_files.clear();
        }
    }

    /// Définit un message flash qui s'affichera pendant 3 secondes.
    pub fn set_flash_message(&mut self, message: String) {
        self.flash_message = Some((message, Instant::now()));
    }

    /// Vérifie si le message flash a expiré et le supprime le cas échéant.
    pub fn check_flash_expired(&mut self) {
        if let Some((_, timestamp)) = &self.flash_message {
            if timestamp.elapsed() > Duration::from_secs(3) {
                self.flash_message = None;
            }
        }
    }

    /// Retourne le commit actuellement sélectionné.
    pub fn selected_commit(&self) -> Option<&crate::git::graph::CommitNode> {
        self.graph.get(self.selected_index).map(|row| &row.node)
    }

    /// Applique une action à l'état de l'application.
    pub fn apply_action(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Quit => {
                self.should_quit = true;
            }
            AppAction::MoveUp => {
                if self.show_branch_panel {
                    if self.branch_selected > 0 {
                        self.branch_selected -= 1;
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.graph_state.select(Some(self.selected_index * 2));
                    self.update_commit_files();
                }
            }
            AppAction::MoveDown => {
                if self.show_branch_panel {
                    if self.branch_selected + 1 < self.branches.len() {
                        self.branch_selected += 1;
                    }
                } else if self.selected_index + 1 < self.graph.len() {
                    self.selected_index += 1;
                    self.graph_state.select(Some(self.selected_index * 2));
                    self.update_commit_files();
                }
            }
            AppAction::PageUp => {
                if !self.show_branch_panel && !self.graph.is_empty() {
                    let page_size = 10;
                    self.selected_index = self.selected_index.saturating_sub(page_size);
                    self.graph_state.select(Some(self.selected_index * 2));
                    self.update_commit_files();
                }
            }
            AppAction::PageDown => {
                if !self.show_branch_panel && !self.graph.is_empty() {
                    let page_size = 10;
                    self.selected_index =
                        (self.selected_index + page_size).min(self.graph.len() - 1);
                    self.graph_state.select(Some(self.selected_index * 2));
                    self.update_commit_files();
                }
            }
            AppAction::GoTop => {
                if !self.show_branch_panel && !self.graph.is_empty() {
                    self.selected_index = 0;
                    self.graph_state.select(Some(0));
                    self.update_commit_files();
                }
            }
            AppAction::GoBottom => {
                if !self.show_branch_panel && !self.graph.is_empty() {
                    self.selected_index = self.graph.len() - 1;
                    self.graph_state.select(Some(self.selected_index * 2));
                    self.update_commit_files();
                }
            }
            AppAction::Select => {
                // Pour l'instant, Select ne fait rien de spécial.
                // Plus tard : ouvrir un panneau de détail étendu.
            }
            AppAction::Refresh => {
                self.refresh()?;
            }
            AppAction::ToggleHelp => {
                self.view_mode = if self.view_mode == ViewMode::Help {
                    ViewMode::Graph
                } else {
                    ViewMode::Help
                };
            }
            AppAction::SwitchBottomMode => {
                // Cycle entre les panneaux : Graph -> Files -> Detail -> Graph
                self.focus = match self.focus {
                    FocusPanel::Graph => FocusPanel::Files,
                    FocusPanel::Files => FocusPanel::Detail,
                    FocusPanel::Detail => FocusPanel::Graph,
                };
            }
            AppAction::BranchList => {
                if self.show_branch_panel {
                    self.show_branch_panel = false;
                } else {
                    self.branches = self.repo.branches().unwrap_or_default();
                    self.branch_selected = 0;
                    self.show_branch_panel = true;
                }
            }
            AppAction::CloseBranchPanel => {
                self.show_branch_panel = false;
            }
            AppAction::BranchCheckout => {
                if self.show_branch_panel {
                    if let Some(branch) = self.branches.get(self.branch_selected).cloned() {
                        if let Err(e) = self.repo.checkout_branch(&branch.name) {
                            self.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.show_branch_panel = false;
                            self.refresh()?;
                            self.set_flash_message(format!("Checkout sur '{}'", branch.name));
                        }
                    }
                }
            }
            AppAction::BranchCreate => {
                // TODO: implémenter le prompt pour créer une branche
            }
            AppAction::BranchDelete => {
                // TODO: implémenter la confirmation et suppression
            }
            // Les prompts seront implémentés dans les prochaines itérations.
            AppAction::CommitPrompt | AppAction::StashPrompt | AppAction::MergePrompt => {
                // TODO: implémenter les modales/prompts interactifs.
            }
        }
        Ok(())
    }

    /// Lance la boucle événementielle principale de l'application.
    pub fn run(&mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;

        let result = self.event_loop(&mut terminal);

        restore_terminal(&mut terminal)?;
        result
    }

    /// Boucle événementielle : render -> poll input -> update.
    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            // Render.
            terminal.draw(|frame| {
                ui::render(
                    frame,
                    &self.graph,
                    &self.current_branch,
                    &self.commit_files,
                    &self.status_entries,
                    &self.branches,
                    self.selected_index,
                    self.branch_selected,
                    self.bottom_left_mode.clone(),
                    self.focus.clone(),
                    &mut self.graph_state,
                    self.view_mode.clone(),
                    self.show_branch_panel,
                    &self.repo_path,
                    self.flash_message.as_ref().map(|(msg, _)| msg.as_str()),
                );
            })?;

            // Input.
            if let Some(action) = ui::input::handle_input(self)? {
                self.apply_action(action)?;
            }

            if self.should_quit {
                break;
            }

            // Vérifier si le message flash a expiré.
            self.check_flash_expired();
        }
        Ok(())
    }
}

/// Initialise le terminal en mode raw + alternate screen.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restaure le terminal à son état normal.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
