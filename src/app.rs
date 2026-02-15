use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::git::branch::BranchInfo;
use crate::git::diff::{DiffFile, FileDiff};
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
    /// Naviguer vers le haut dans le panneau de fichiers.
    FileUp,
    /// Naviguer vers le bas dans le panneau de fichiers.
    FileDown,
    /// Scroller vers le haut dans le diff.
    DiffScrollUp,
    /// Scroller vers le bas dans le diff.
    DiffScrollDown,
    /// Basculer vers la vue Graph.
    SwitchToGraph,
    /// Basculer vers la vue Staging.
    SwitchToStaging,
    /// Basculer vers la vue Branches.
    SwitchToBranches,
    /// Stage le fichier sélectionné.
    StageFile,
    /// Unstage le fichier sélectionné.
    UnstageFile,
    /// Stage tous les fichiers.
    StageAll,
    /// Unstage tous les fichiers.
    UnstageAll,
    /// Changer le focus dans la vue staging.
    SwitchStagingFocus,
    /// Activer le mode saisie de message de commit.
    StartCommitMessage,
    /// Valider le commit.
    ConfirmCommit,
    /// Annuler la saisie du message.
    CancelCommitMessage,
    /// Insérer un caractère dans le message de commit.
    InsertChar(char),
    /// Supprimer un caractère dans le message de commit.
    DeleteChar,
    /// Déplacer le curseur à gauche.
    MoveCursorLeft,
    /// Déplacer le curseur à droite.
    MoveCursorRight,
    /// Basculer vers la section suivante (Branches → Worktrees → Stashes).
    NextSection,
    /// Basculer vers la section précédente.
    PrevSection,
    /// Renommer la branche sélectionnée (ouvre input).
    BranchRename,
    /// Toggle affichage branches remote.
    ToggleRemoteBranches,
    /// Créer un worktree (ouvre input).
    WorktreeCreate,
    /// Supprimer le worktree sélectionné.
    WorktreeRemove,
    /// Appliquer le stash sélectionné (sans supprimer).
    StashApply,
    /// Pop le stash sélectionné (appliquer + supprimer).
    StashPop,
    /// Supprimer le stash sélectionné.
    StashDrop,
    /// Créer un nouveau stash (ouvre input).
    StashSave,
    /// Confirmer l'input.
    ConfirmInput,
    /// Annuler l'input.
    CancelInput,
}

/// Mode d'affichage actif.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Graph,
    Help,
    Staging,
    Branches,
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

/// Panneau focalisé dans la vue staging.
#[derive(Debug, Clone, PartialEq)]
pub enum StagingFocus {
    /// Liste des fichiers non staged (working directory).
    Unstaged,
    /// Liste des fichiers staged (index).
    Staged,
    /// Panneau de diff (droite).
    Diff,
    /// Champ de saisie du message de commit.
    CommitMessage,
}

/// État de la vue staging.
pub struct StagingState {
    /// Fichiers non staged.
    pub unstaged_files: Vec<StatusEntry>,
    /// Fichiers staged.
    pub staged_files: Vec<StatusEntry>,
    /// Index sélectionné dans le panneau unstaged.
    pub unstaged_selected: usize,
    /// Index sélectionné dans le panneau staged.
    pub staged_selected: usize,
    /// Panneau actuellement focalisé.
    pub focus: StagingFocus,
    /// Diff du fichier survolé.
    pub current_diff: Option<FileDiff>,
    /// Offset de scroll dans le diff.
    pub diff_scroll: usize,
    /// Message de commit en cours de saisie.
    pub commit_message: String,
    /// Position du curseur dans le message.
    pub cursor_position: usize,
    /// Mode saisie de message activé.
    pub is_committing: bool,
}

impl Default for StagingState {
    fn default() -> Self {
        Self {
            unstaged_files: Vec::new(),
            staged_files: Vec::new(),
            unstaged_selected: 0,
            staged_selected: 0,
            focus: StagingFocus::Unstaged,
            current_diff: None,
            diff_scroll: 0,
            commit_message: String::new(),
            cursor_position: 0,
            is_committing: false,
        }
    }
}

/// Section active dans la vue branches.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchesSection {
    Branches,
    Worktrees,
    Stashes,
}

/// Panneau focalisé dans la vue branches.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchesFocus {
    List,
    Detail,
    Input,
}

/// Action d'input en cours.
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    CreateBranch,
    CreateWorktree,
    RenameBranch,
    SaveStash,
}

/// État de la vue branches/worktree/stash.
pub struct BranchesViewState {
    pub section: BranchesSection,
    pub focus: BranchesFocus,
    pub local_branches: Vec<BranchInfo>,
    pub remote_branches: Vec<BranchInfo>,
    pub branch_selected: usize,
    pub show_remote: bool,
    pub worktrees: Vec<crate::git::worktree::WorktreeInfo>,
    pub worktree_selected: usize,
    pub stashes: Vec<crate::git::stash::StashEntry>,
    pub stash_selected: usize,
    pub input_text: String,
    pub input_cursor: usize,
    pub input_action: Option<InputAction>,
}

impl Default for BranchesViewState {
    fn default() -> Self {
        Self {
            section: BranchesSection::Branches,
            focus: BranchesFocus::List,
            local_branches: Vec::new(),
            remote_branches: Vec::new(),
            branch_selected: 0,
            show_remote: false,
            worktrees: Vec::new(),
            worktree_selected: 0,
            stashes: Vec::new(),
            stash_selected: 0,
            input_text: String::new(),
            input_cursor: 0,
            input_action: None,
        }
    }
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
    /// Index du fichier sélectionné dans le panneau de fichiers.
    pub file_selected_index: usize,
    /// Diff du fichier sélectionné (chargé à la demande).
    pub selected_file_diff: Option<FileDiff>,
    /// Offset de scroll dans le panneau de diff.
    pub diff_scroll_offset: usize,
    /// État de la vue staging.
    pub staging_state: StagingState,
    /// État de la vue branches.
    pub branches_view_state: BranchesViewState,
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
            file_selected_index: 0,
            selected_file_diff: None,
            diff_scroll_offset: 0,
            staging_state: StagingState::default(),
            branches_view_state: BranchesViewState::default(),
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

        // Réinitialiser la sélection de fichier.
        self.file_selected_index = 0;
        self.selected_file_diff = None;
        self.diff_scroll_offset = 0;

        // Rafraîchir aussi l'état de staging.
        self.refresh_staging()?;

        Ok(())
    }

    /// Rafraîchit l'état de la vue staging.
    fn refresh_staging(&mut self) -> Result<()> {
        let all_entries = self.repo.status().unwrap_or_default();

        self.staging_state.staged_files = all_entries
            .iter()
            .filter(|e| e.is_staged())
            .cloned()
            .collect();

        self.staging_state.unstaged_files = all_entries
            .iter()
            .filter(|e| e.is_unstaged())
            .cloned()
            .collect();

        // Réajuster les sélections.
        if self.staging_state.unstaged_selected >= self.staging_state.unstaged_files.len() {
            self.staging_state.unstaged_selected =
                self.staging_state.unstaged_files.len().saturating_sub(1);
        }
        if self.staging_state.staged_selected >= self.staging_state.staged_files.len() {
            self.staging_state.staged_selected =
                self.staging_state.staged_files.len().saturating_sub(1);
        }

        // Charger le diff du fichier survolé.
        self.load_staging_diff();

        Ok(())
    }

    /// Charge le diff du fichier sélectionné dans la vue staging.
    fn load_staging_diff(&mut self) {
        let selected_file = match self.staging_state.focus {
            StagingFocus::Unstaged => self
                .staging_state
                .unstaged_files
                .get(self.staging_state.unstaged_selected),
            StagingFocus::Staged => self
                .staging_state
                .staged_files
                .get(self.staging_state.staged_selected),
            _ => None,
        };

        if let Some(file) = selected_file {
            // Utiliser le working directory diff pour les fichiers unstaged
            // et le HEAD diff pour les fichiers staged
            self.staging_state.current_diff =
                crate::git::diff::working_dir_file_diff(&self.repo.repo, &file.path).ok();
        } else {
            self.staging_state.current_diff = None;
        }
        self.staging_state.diff_scroll = 0;
    }

    /// Met à jour la liste des fichiers du commit sélectionné.
    fn update_commit_files(&mut self) {
        if let Some(row) = self.graph.get(self.selected_index) {
            self.commit_files = self.repo.commit_diff(row.node.oid).unwrap_or_default();
        } else {
            self.commit_files.clear();
        }
        // Réinitialiser le diff sélectionné quand on change de commit.
        self.file_selected_index = 0;
        self.selected_file_diff = None;
        self.diff_scroll_offset = 0;
    }

    /// Charge le diff du fichier sélectionné.
    fn load_selected_file_diff(&mut self) {
        if let Some(file) = self.commit_files.get(self.file_selected_index) {
            if let Some(row) = self.graph.get(self.selected_index) {
                self.selected_file_diff = self.repo.file_diff(row.node.oid, &file.path).ok();
            }
        } else {
            self.selected_file_diff = None;
        }
        self.diff_scroll_offset = 0;
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
                } else if self.view_mode == ViewMode::Staging {
                    self.handle_staging_navigation(-1);
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
                } else if self.view_mode == ViewMode::Staging {
                    self.handle_staging_navigation(1);
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
            AppAction::FileUp => {
                if self.focus == FocusPanel::Files && !self.commit_files.is_empty() {
                    if self.file_selected_index > 0 {
                        self.file_selected_index -= 1;
                        self.load_selected_file_diff();
                    }
                }
            }
            AppAction::FileDown => {
                if self.focus == FocusPanel::Files && !self.commit_files.is_empty() {
                    if self.file_selected_index + 1 < self.commit_files.len() {
                        self.file_selected_index += 1;
                        self.load_selected_file_diff();
                    }
                }
            }
            AppAction::DiffScrollUp => {
                if self.focus == FocusPanel::Detail && self.diff_scroll_offset > 0 {
                    self.diff_scroll_offset -= 1;
                } else if self.view_mode == ViewMode::Staging && self.staging_state.diff_scroll > 0
                {
                    self.staging_state.diff_scroll -= 1;
                }
            }
            AppAction::DiffScrollDown => {
                if self.focus == FocusPanel::Detail {
                    self.diff_scroll_offset += 1;
                } else if self.view_mode == ViewMode::Staging {
                    self.staging_state.diff_scroll += 1;
                }
            }
            // Actions de changement de vue
            AppAction::SwitchToGraph => {
                self.view_mode = ViewMode::Graph;
                self.refresh()?;
            }
            AppAction::SwitchToStaging => {
                self.view_mode = ViewMode::Staging;
                self.refresh_staging()?;
            }
            AppAction::SwitchToBranches => {
                self.view_mode = ViewMode::Branches;
                self.refresh_branches_view()?;
            }
            // Actions de staging
            AppAction::StageFile => {
                if self.view_mode == ViewMode::Staging {
                    if let Some(file) = self
                        .staging_state
                        .unstaged_files
                        .get(self.staging_state.unstaged_selected)
                    {
                        crate::git::commit::stage_file(&self.repo.repo, &file.path)?;
                        self.refresh_staging()?;
                    }
                }
            }
            AppAction::UnstageFile => {
                if self.view_mode == ViewMode::Staging {
                    if let Some(file) = self
                        .staging_state
                        .staged_files
                        .get(self.staging_state.staged_selected)
                    {
                        crate::git::commit::unstage_file(&self.repo.repo, &file.path)?;
                        self.refresh_staging()?;
                    }
                }
            }
            AppAction::StageAll => {
                if self.view_mode == ViewMode::Staging {
                    crate::git::commit::stage_all(&self.repo.repo)?;
                    self.refresh_staging()?;
                }
            }
            AppAction::UnstageAll => {
                if self.view_mode == ViewMode::Staging {
                    crate::git::commit::unstage_all(&self.repo.repo)?;
                    self.refresh_staging()?;
                }
            }
            AppAction::SwitchStagingFocus => {
                if self.view_mode == ViewMode::Staging {
                    self.staging_state.focus = match self.staging_state.focus {
                        StagingFocus::Unstaged => StagingFocus::Staged,
                        StagingFocus::Staged => StagingFocus::Diff,
                        StagingFocus::Diff => StagingFocus::Unstaged,
                        StagingFocus::CommitMessage => StagingFocus::Unstaged,
                    };
                    self.load_staging_diff();
                }
            }
            AppAction::StartCommitMessage => {
                if self.view_mode == ViewMode::Staging {
                    self.staging_state.is_committing = true;
                    self.staging_state.focus = StagingFocus::CommitMessage;
                }
            }
            AppAction::ConfirmCommit => {
                if self.view_mode == ViewMode::Staging
                    && !self.staging_state.commit_message.is_empty()
                    && !self.staging_state.staged_files.is_empty()
                {
                    crate::git::commit::create_commit(
                        &self.repo.repo,
                        &self.staging_state.commit_message,
                    )?;
                    self.staging_state.commit_message.clear();
                    self.staging_state.cursor_position = 0;
                    self.staging_state.is_committing = false;
                    self.refresh_staging()?;
                    self.set_flash_message("Commit créé avec succès".into());
                }
            }
            AppAction::CancelCommitMessage => {
                if self.view_mode == ViewMode::Staging {
                    self.staging_state.is_committing = false;
                    self.staging_state.focus = StagingFocus::Unstaged;
                }
            }
            AppAction::InsertChar(c) => {
                if self.view_mode == ViewMode::Staging && self.staging_state.is_committing {
                    self.staging_state
                        .commit_message
                        .insert(self.staging_state.cursor_position, c);
                    self.staging_state.cursor_position += 1;
                }
            }
            AppAction::DeleteChar => {
                if self.view_mode == ViewMode::Staging
                    && self.staging_state.is_committing
                    && self.staging_state.cursor_position > 0
                {
                    self.staging_state.cursor_position -= 1;
                    self.staging_state
                        .commit_message
                        .remove(self.staging_state.cursor_position);
                }
            }
            AppAction::MoveCursorLeft => {
                if self.view_mode == ViewMode::Staging
                    && self.staging_state.is_committing
                    && self.staging_state.cursor_position > 0
                {
                    self.staging_state.cursor_position -= 1;
                }
            }
            AppAction::MoveCursorRight => {
                if self.view_mode == ViewMode::Staging
                    && self.staging_state.is_committing
                    && self.staging_state.cursor_position < self.staging_state.commit_message.len()
                {
                    self.staging_state.cursor_position += 1;
                }
            }
            // Actions de la vue Branches
            AppAction::NextSection => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.section = match self.branches_view_state.section {
                        BranchesSection::Branches => BranchesSection::Worktrees,
                        BranchesSection::Worktrees => BranchesSection::Stashes,
                        BranchesSection::Stashes => BranchesSection::Branches,
                    };
                }
            }
            AppAction::PrevSection => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.section = match self.branches_view_state.section {
                        BranchesSection::Branches => BranchesSection::Stashes,
                        BranchesSection::Worktrees => BranchesSection::Branches,
                        BranchesSection::Stashes => BranchesSection::Worktrees,
                    };
                }
            }
            AppAction::BranchRename => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.focus = BranchesFocus::Input;
                    self.branches_view_state.input_action = Some(InputAction::RenameBranch);
                    self.branches_view_state.input_text.clear();
                    self.branches_view_state.input_cursor = 0;
                }
            }
            AppAction::ToggleRemoteBranches => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.show_remote = !self.branches_view_state.show_remote;
                }
            }
            AppAction::WorktreeCreate => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.focus = BranchesFocus::Input;
                    self.branches_view_state.input_action = Some(InputAction::CreateWorktree);
                    self.branches_view_state.input_text.clear();
                    self.branches_view_state.input_cursor = 0;
                }
            }
            AppAction::WorktreeRemove => {
                if self.view_mode == ViewMode::Branches {
                    if let Some(worktree) = self
                        .branches_view_state
                        .worktrees
                        .get(self.branches_view_state.worktree_selected)
                    {
                        if !worktree.is_main {
                            let name = worktree.name.clone();
                            if let Err(e) = self.repo.remove_worktree(&name) {
                                self.set_flash_message(format!("Erreur: {}", e));
                            } else {
                                self.set_flash_message(format!("Worktree '{}' supprimé", name));
                                self.refresh_branches_view()?;
                            }
                        } else {
                            self.set_flash_message(
                                "Impossible de supprimer le worktree principal".into(),
                            );
                        }
                    }
                }
            }
            AppAction::StashApply => {
                if self.view_mode == ViewMode::Branches {
                    if let Some(stash) = self
                        .branches_view_state
                        .stashes
                        .get(self.branches_view_state.stash_selected)
                    {
                        let idx = stash.index;
                        if let Err(e) = crate::git::stash::apply_stash(&mut self.repo.repo, idx) {
                            self.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.set_flash_message(format!("Stash @{{{}}} appliqué", idx));
                            self.refresh_branches_view()?;
                        }
                    }
                }
            }
            AppAction::StashPop => {
                if self.view_mode == ViewMode::Branches {
                    if let Some(stash) = self
                        .branches_view_state
                        .stashes
                        .get(self.branches_view_state.stash_selected)
                    {
                        let idx = stash.index;
                        if let Err(e) = crate::git::stash::pop_stash(&mut self.repo.repo, idx) {
                            self.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.set_flash_message(format!(
                                "Stash @{{{}}} appliqué et supprimé",
                                idx
                            ));
                            self.refresh_branches_view()?;
                        }
                    }
                }
            }
            AppAction::StashDrop => {
                if self.view_mode == ViewMode::Branches {
                    if let Some(stash) = self
                        .branches_view_state
                        .stashes
                        .get(self.branches_view_state.stash_selected)
                    {
                        let idx = stash.index;
                        if let Err(e) = crate::git::stash::drop_stash(&mut self.repo.repo, idx) {
                            self.set_flash_message(format!("Erreur: {}", e));
                        } else {
                            self.set_flash_message(format!("Stash @{{{}}} supprimé", idx));
                            self.refresh_branches_view()?;
                        }
                    }
                }
            }
            AppAction::StashSave => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.focus = BranchesFocus::Input;
                    self.branches_view_state.input_action = Some(InputAction::SaveStash);
                    self.branches_view_state.input_text.clear();
                    self.branches_view_state.input_cursor = 0;
                }
            }
            AppAction::ConfirmInput => {
                if self.view_mode == ViewMode::Branches
                    && self.branches_view_state.focus == BranchesFocus::Input
                {
                    match self.branches_view_state.input_action.take() {
                        Some(InputAction::CreateBranch) => {
                            let name = self.branches_view_state.input_text.clone();
                            if !name.is_empty() {
                                if let Err(e) =
                                    crate::git::branch::create_branch(&self.repo.repo, &name)
                                {
                                    self.set_flash_message(format!("Erreur: {}", e));
                                } else {
                                    self.set_flash_message(format!("Branche '{}' créée", name));
                                    self.refresh_branches_view()?;
                                }
                            }
                        }
                        Some(InputAction::RenameBranch) => {
                            if let Some(branch) = self
                                .branches_view_state
                                .local_branches
                                .get(self.branches_view_state.branch_selected)
                            {
                                let old_name = branch.name.clone();
                                let new_name = self.branches_view_state.input_text.clone();
                                if !new_name.is_empty() && new_name != old_name {
                                    if let Err(e) = crate::git::branch::rename_branch(
                                        &self.repo.repo,
                                        &old_name,
                                        &new_name,
                                    ) {
                                        self.set_flash_message(format!("Erreur: {}", e));
                                    } else {
                                        self.set_flash_message(format!(
                                            "Branche '{}' renommée en '{}'",
                                            old_name, new_name
                                        ));
                                        self.refresh_branches_view()?;
                                    }
                                }
                            }
                        }
                        Some(InputAction::CreateWorktree) => {
                            let input = self.branches_view_state.input_text.clone();
                            // Format attendu: "nom chemin [branche]"
                            let parts: Vec<&str> = input.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let name = parts[0];
                                let path = parts[1];
                                let branch = parts.get(2).map(|s| *s);
                                if let Err(e) = self.repo.create_worktree(name, path, branch) {
                                    self.set_flash_message(format!("Erreur: {}", e));
                                } else {
                                    self.set_flash_message(format!("Worktree '{}' créé", name));
                                    self.refresh_branches_view()?;
                                }
                            } else {
                                self.set_flash_message("Format: nom chemin [branche]".into());
                            }
                        }
                        Some(InputAction::SaveStash) => {
                            let msg = self.branches_view_state.input_text.clone();
                            let msg_opt = if msg.is_empty() {
                                None
                            } else {
                                Some(msg.as_str())
                            };
                            if let Err(e) =
                                crate::git::stash::save_stash(&mut self.repo.repo, msg_opt)
                            {
                                self.set_flash_message(format!("Erreur: {}", e));
                            } else {
                                self.set_flash_message("Stash sauvegardé".into());
                                self.refresh_branches_view()?;
                            }
                        }
                        _ => {}
                    }
                    self.branches_view_state.focus = BranchesFocus::List;
                    self.branches_view_state.input_text.clear();
                    self.branches_view_state.input_cursor = 0;
                }
            }
            AppAction::CancelInput => {
                if self.view_mode == ViewMode::Branches {
                    self.branches_view_state.focus = BranchesFocus::List;
                    self.branches_view_state.input_action = None;
                    self.branches_view_state.input_text.clear();
                    self.branches_view_state.input_cursor = 0;
                }
            }
            // Les prompts seront implémentés dans les prochaines itérations.
            AppAction::CommitPrompt | AppAction::StashPrompt | AppAction::MergePrompt => {
                // TODO: implémenter les modales/prompts interactifs.
            }
        }
        Ok(())
    }

    /// Gère la navigation dans la vue staging.
    fn handle_staging_navigation(&mut self, direction: i32) {
        match self.staging_state.focus {
            StagingFocus::Unstaged => {
                let max = self.staging_state.unstaged_files.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.staging_state.unstaged_selected + 1).min(max - 1)
                    } else {
                        self.staging_state.unstaged_selected.saturating_sub(1)
                    };
                    self.staging_state.unstaged_selected = new_idx;
                    self.load_staging_diff();
                }
            }
            StagingFocus::Staged => {
                let max = self.staging_state.staged_files.len();
                if max > 0 {
                    let new_idx = if direction > 0 {
                        (self.staging_state.staged_selected + 1).min(max - 1)
                    } else {
                        self.staging_state.staged_selected.saturating_sub(1)
                    };
                    self.staging_state.staged_selected = new_idx;
                    self.load_staging_diff();
                }
            }
            StagingFocus::Diff => {
                // Navigation dans le diff avec scroll
                if direction > 0 {
                    self.staging_state.diff_scroll += 1;
                } else if self.staging_state.diff_scroll > 0 {
                    self.staging_state.diff_scroll -= 1;
                }
            }
            _ => {}
        }
    }

    /// Rafraîchit l'état de la vue branches.
    fn refresh_branches_view(&mut self) -> Result<()> {
        let (local_branches, remote_branches) =
            crate::git::branch::list_all_branches(&self.repo.repo)?;

        self.branches_view_state.local_branches = local_branches;
        self.branches_view_state.remote_branches = remote_branches;
        self.branches_view_state.worktrees = self.repo.worktrees().unwrap_or_default();
        self.branches_view_state.stashes = self.repo.stashes().unwrap_or_default();

        // Réajuster les sélections.
        if self.branches_view_state.branch_selected >= self.branches_view_state.local_branches.len()
        {
            self.branches_view_state.branch_selected = self
                .branches_view_state
                .local_branches
                .len()
                .saturating_sub(1);
        }
        if self.branches_view_state.worktree_selected >= self.branches_view_state.worktrees.len() {
            self.branches_view_state.worktree_selected =
                self.branches_view_state.worktrees.len().saturating_sub(1);
        }
        if self.branches_view_state.stash_selected >= self.branches_view_state.stashes.len() {
            self.branches_view_state.stash_selected =
                self.branches_view_state.stashes.len().saturating_sub(1);
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
                    self.file_selected_index,
                    self.selected_file_diff.as_ref(),
                    self.diff_scroll_offset,
                    &self.staging_state,
                    &self.branches_view_state,
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
