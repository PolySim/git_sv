use crate::git::branch::BranchInfo;
use crate::git::diff::{DiffFile, FileDiff};
use crate::git::graph::GraphRow;
use crate::git::repo::{GitRepo, StatusEntry};
use crate::git::stash::StashEntry;
use crate::git::worktree::WorktreeInfo;
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

/// Nombre maximum de commits à charger.
pub const MAX_COMMITS: usize = 200;

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
    pub worktrees: Vec<WorktreeInfo>,
    pub worktree_selected: usize,
    pub stashes: Vec<StashEntry>,
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
pub struct AppState {
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
    pub file_selected_index: usize,
    pub selected_file_diff: Option<FileDiff>,
    pub diff_scroll_offset: usize,
    pub staging_state: StagingState,
    pub branches_view_state: BranchesViewState,
}

impl AppState {
    /// Crée un nouvel état d'application.
    pub fn new(repo: GitRepo, repo_path: String) -> crate::error::Result<Self> {
        let mut graph_state = ListState::default();
        graph_state.select(Some(0));

        let state = Self {
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

        Ok(state)
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

    /// Retourne le message flash actuel s'il n'a pas expiré.
    pub fn current_flash_message(&self) -> Option<&str> {
        self.flash_message.as_ref().map(|(msg, _)| msg.as_str())
    }
}
