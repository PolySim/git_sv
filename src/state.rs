use crate::git::branch::BranchInfo;
use crate::git::diff::{DiffFile, FileDiff};
use crate::git::graph::GraphRow;
use crate::git::repo::{GitRepo, StatusEntry};
use crate::git::stash::StashEntry;
use crate::git::worktree::WorktreeInfo;
use crate::ui::confirm_dialog::ConfirmAction;
use crate::ui::loading::LoadingSpinner;
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
    /// Confirmer une action destructive (oui).
    ConfirmAction,
    /// Annuler une action destructive (non).
    CancelAction,
    /// Pousser la branche courante vers le remote.
    GitPush,
    /// Tirer les changements depuis le remote.
    GitPull,
    /// Récupérer les refs du remote sans merger.
    GitFetch,
    /// Ouvrir le mode recherche.
    OpenSearch,
    /// Fermer le mode recherche.
    CloseSearch,
    /// Changer le type de recherche (message/auteur/hash).
    ChangeSearchType,
    /// Aller au résultat suivant.
    NextSearchResult,
    /// Aller au résultat précédent.
    PrevSearchResult,
    /// Discard les modifications d'un fichier.
    DiscardFile,
    /// Discard toutes les modifications non stagées.
    DiscardAll,
}

/// Mode d'affichage actif.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Graph,
    Help,
    Staging,
    Branches,
}

/// Mode du panneau bas-gauche.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BottomLeftMode {
    CommitFiles,
    WorkingDir,
}

/// Panneau actuellement focalisé.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPanel {
    Graph,
    Files,
    Detail,
}

/// Panneau focalisé dans la vue staging.
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchesSection {
    Branches,
    Worktrees,
    Stashes,
}

/// Panneau focalisé dans la vue branches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchesFocus {
    List,
    Detail,
    Input,
}

/// Action d'input en cours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputAction {
    CreateBranch,
    CreateWorktree,
    RenameBranch,
    SaveStash,
    MergeBranch,
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

/// État de la recherche de commits.
pub struct SearchState {
    /// Recherche activée ou non.
    pub is_active: bool,
    /// Texte de recherche.
    pub query: String,
    /// Position du curseur dans le texte de recherche.
    pub cursor: usize,
    /// Type de recherche en cours.
    pub search_type: crate::git::search::SearchType,
    /// Indices des commits correspondant à la recherche.
    pub results: Vec<usize>,
    /// Index du résultat actuellement sélectionné dans results.
    pub current_result: usize,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            is_active: false,
            query: String::new(),
            cursor: 0,
            search_type: crate::git::search::SearchType::Message,
            results: Vec::new(),
            current_result: 0,
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
    /// État de la recherche de commits.
    pub search_state: SearchState,
    /// Flag indiquant si les données ont changé et nécessitent un rafraîchissement.
    pub dirty: bool,
    /// Cache pour les diffs de fichiers (LRU simple).
    pub diff_cache: DiffCache,
    /// Action en attente de confirmation (dialogue modal).
    pub pending_confirmation: Option<ConfirmAction>,
    /// Spinner de chargement actif (None si pas de chargement).
    pub loading_spinner: Option<LoadingSpinner>,
}

/// Cache LRU simple pour les diffs de fichiers.
/// Clé: (Oid du commit, chemin du fichier)
/// Valeur: FileDiff
pub struct DiffCache {
    cache: std::collections::HashMap<(git2::Oid, String), crate::git::diff::FileDiff>,
    /// Ordre d'accès pour LRU (dernier = plus récent).
    access_order: Vec<(git2::Oid, String)>,
    /// Taille maximale du cache.
    max_size: usize,
}

impl DiffCache {
    /// Crée un nouveau cache avec une taille maximale.
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            access_order: Vec::new(),
            max_size,
        }
    }

    /// Récupère un diff du cache.
    pub fn get(&mut self, key: &(git2::Oid, String)) -> Option<&crate::git::diff::FileDiff> {
        if self.cache.contains_key(key) {
            // Mettre à jour l'ordre d'accès (LRU)
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key = self.access_order.remove(pos);
                self.access_order.push(key);
            }
            self.cache.get(key)
        } else {
            None
        }
    }

    /// Insère un diff dans le cache.
    pub fn insert(&mut self, key: (git2::Oid, String), value: crate::git::diff::FileDiff) {
        // Si la clé existe déjà, mettre à jour juste la valeur
        if self.cache.contains_key(&key) {
            self.cache.insert(key.clone(), value);
            // Mettre à jour l'ordre d'accès
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                let key = self.access_order.remove(pos);
                self.access_order.push(key);
            }
            return;
        }

        // Éviction LRU si nécessaire
        if self.cache.len() >= self.max_size && !self.access_order.is_empty() {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.cache.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        self.cache.insert(key.clone(), value);
        self.access_order.push(key);
    }

    /// Vide le cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Supprime les entrées du working directory (Oid::zero()).
    pub fn clear_working_directory(&mut self) {
        let to_remove: Vec<_> = self
            .cache
            .keys()
            .filter(|(oid, _)| *oid == git2::Oid::zero())
            .cloned()
            .collect();
        for key in to_remove {
            self.cache.remove(&key);
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                self.access_order.remove(pos);
            }
        }
    }
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
            search_state: SearchState::default(),
            dirty: true,                    // Initialement dirty pour charger les données
            diff_cache: DiffCache::new(50), // Cache de 50 diffs
            pending_confirmation: None,
            loading_spinner: None,
        };

        Ok(state)
    }

    /// Marque l'état comme modifié (dirty).
    /// Vide aussi le cache des diffs du working directory car les opérations git peuvent les invalider.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        // Vider le cache des diffs du working directory car ils peuvent être invalidés
        // Les diffs de commits historiques peuvent rester en cache
        self.diff_cache.clear_working_directory();
    }

    /// Marque l'état comme à jour (not dirty).
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
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
