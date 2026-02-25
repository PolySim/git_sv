//! État global de l'application.

pub mod action;
pub mod cache;
pub mod filter;
pub mod selection;
pub mod view;

pub use action::AppAction;
pub use cache::{DiffCache, DiffCacheKey, LazyBlame, LazyDiff};
pub use filter::{FilterField, FilterPopupState, GraphFilter};
pub use selection::ListSelection;
pub use view::*;

use crate::git::branch::BranchInfo;
use crate::git::diff::{DiffFile, DiffViewMode};
use crate::git::graph::GraphRow;
use crate::git::repo::{GitRepo, StatusEntry};
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

/// Nombre maximum de commits à charger.
pub const MAX_COMMITS: usize = 200;

/// État principal de l'application.
pub struct AppState {
    // ═══════════════════════════════════════════════════
    // Core
    // ═══════════════════════════════════════════════════
    /// Repository git.
    pub repo: GitRepo,

    /// Chemin du repository.
    pub repo_path: String,

    /// Branche courante.
    pub current_branch: Option<String>,

    // ═══════════════════════════════════════════════════
    // Vue active
    // ═══════════════════════════════════════════════════
    /// Mode de vue actuel.
    pub view_mode: ViewMode,

    /// Mode de vue précédent (pour retour depuis Help).
    pub previous_view_mode: Option<ViewMode>,

    /// État indiquant si un refresh est nécessaire.
    pub dirty: bool,

    // ═══════════════════════════════════════════════════
    // Vue Graph (toujours chargée)
    // ═══════════════════════════════════════════════════
    /// Lignes du graph de commits (compatibilité - migrer vers graph_view.rows).
    pub graph: Vec<GraphRow>,

    /// État de la vue graph avec sélection générique.
    pub graph_view: GraphViewState,

    /// Mode d'affichage du panneau bottom-left.
    pub bottom_left_mode: BottomLeftMode,

    /// Panneau avec focus.
    pub focus: FocusPanel,

    /// Index sélectionné (compatibilité - migrer vers graph_view.rows.selected_index()).
    pub selected_index: usize,

    /// État de la liste pour ratatui (compatibilité).
    pub graph_state: ListState,

    // ═══════════════════════════════════════════════════
    // Données associées au commit sélectionné (compatibilité)
    // ═══════════════════════════════════════════════════
    /// Fichiers du commit sélectionné.
    pub commit_files: Vec<DiffFile>,

    /// Index du fichier sélectionné (compatibilité - migrer vers graph_view.file_selected_index).
    pub file_selected_index: usize,

    /// Diff du fichier sélectionné.
    pub selected_file_diff: Option<crate::git::diff::FileDiff>,

    /// Offset de scroll dans le diff (compatibilité - migrer vers graph_view.diff_scroll_offset).
    pub diff_scroll_offset: usize,

    /// Mode d'affichage du diff (unifié ou côte à côte).
    pub diff_view_mode: DiffViewMode,

    /// Entrées de status (pour la vue staging, compatibilité).
    pub status_entries: Vec<StatusEntry>,

    /// Branches (compatibilité).
    pub branches: Vec<BranchInfo>,

    // ═══════════════════════════════════════════════════
    // Vues optionnelles (chargées à la demande)
    // ═══════════════════════════════════════════════════
    /// État de la vue staging.
    pub staging_state: StagingState,

    /// État de la vue branches.
    pub branches_view_state: BranchesViewState,

    /// État du blame (si actif).
    pub blame_state: Option<BlameState>,

    /// État de résolution de conflits (si actif).
    pub conflicts_state: Option<ConflictsState>,

    /// État de la recherche.
    pub search_state: SearchState,

    /// Picker de merge (si actif).
    pub merge_picker: Option<MergePickerState>,

    // ═══════════════════════════════════════════════════
    // UI transient
    // ═══════════════════════════════════════════════════
    /// Message flash à afficher.
    pub flash_message: Option<(String, Instant)>,

    /// Action en attente de confirmation (dialogue modal).
    pub pending_confirmation: Option<crate::ui::confirm_dialog::ConfirmAction>,

    /// Spinner de chargement actif.
    pub loading_spinner: Option<crate::ui::loading::LoadingSpinner>,

    /// Panneau de branches ouvert (compatibilité).
    pub show_branch_panel: bool,

    /// Index de la branche sélectionnée dans le panneau (compatibilité).
    pub branch_selected: usize,

    /// Flag pour quitter l'application.
    pub should_quit: bool,

    // ═══════════════════════════════════════════════════
    // Cache
    // ═══════════════════════════════════════════════════
    /// Cache des diffs.
    pub diff_cache: DiffCache,

    // ═══════════════════════════════════════════════════
    // Filtres pour le graph
    // ═══════════════════════════════════════════════════
    /// Filtres actifs sur le graph.
    pub graph_filter: GraphFilter,

    /// État du popup de filtre.
    pub filter_popup: FilterPopupState,
}

impl AppState {
    /// Crée un nouvel état d'application.
    pub fn new(repo: GitRepo, repo_path: String) -> crate::error::Result<Self> {
        let mut graph_state = ListState::default();
        graph_state.select(Some(0));

        let current_branch = repo.current_branch().ok();

        let state = Self {
            repo,
            repo_path,
            current_branch,
            view_mode: ViewMode::Graph,
            previous_view_mode: None,
            dirty: true,
            graph: Vec::new(),
            graph_view: GraphViewState::new(),
            bottom_left_mode: BottomLeftMode::Files,
            focus: FocusPanel::Graph,
            selected_index: 0,
            graph_state,
            commit_files: Vec::new(),
            file_selected_index: 0,
            selected_file_diff: None,
            diff_scroll_offset: 0,
            diff_view_mode: DiffViewMode::default(),
            status_entries: Vec::new(),
            branches: Vec::new(),
            staging_state: StagingState::new(),
            branches_view_state: BranchesViewState::new(),
            blame_state: None,
            conflicts_state: None,
            search_state: SearchState::default(),
            merge_picker: None,
            flash_message: None,
            pending_confirmation: None,
            loading_spinner: None,
            show_branch_panel: false,
            branch_selected: 0,
            should_quit: false,
            diff_cache: DiffCache::new(50),
            graph_filter: GraphFilter::new(),
            filter_popup: FilterPopupState::new(),
        };

        Ok(state)
    }

    /// Marque l'état comme nécessitant un refresh.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        // Vider le cache des diffs du working directory car ils peuvent être invalidés
        self.diff_cache.clear_working_directory();
    }

    /// L'état nécessite-t-il un refresh?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marque l'état comme propre.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Alias de mark_clean pour compatibilité.
    pub fn clear_dirty(&mut self) {
        self.mark_clean();
    }

    /// Définit un message flash.
    pub fn set_flash_message(&mut self, message: impl Into<String>) {
        self.flash_message = Some((message.into(), Instant::now()));
    }

    /// Efface le message flash.
    pub fn clear_flash_message(&mut self) {
        self.flash_message = None;
    }

    /// Vérifie si le message flash a expiré et le supprime le cas échéant.
    pub fn check_flash_expired(&mut self) {
        if let Some((_, timestamp)) = &self.flash_message {
            if timestamp.elapsed() > Duration::from_secs(3) {
                self.flash_message = None;
            }
        }
    }

    /// Retourne le message flash actuel s'il n'a pas expiré.
    pub fn current_flash_message(&self) -> Option<&str> {
        self.flash_message.as_ref().map(|(msg, _)| msg.as_str())
    }

    /// Retourne le commit actuellement sélectionné.
    pub fn selected_commit(&self) -> Option<&crate::git::graph::CommitNode> {
        self.graph.get(self.selected_index).map(|row| &row.node)
    }

    /// Accès au cache de diff.
    pub fn diff_cache_mut(&mut self) -> &mut DiffCache {
        &mut self.diff_cache
    }

    /// Met à jour la sélection du graph à partir de graph_view.
    pub fn sync_graph_selection(&mut self) {
        self.selected_index = self.graph_view.rows.selected_index();
        // Le graphe contient 2 items par commit (ligne + connexion)
        self.graph_state.select(Some(self.selected_index * 2));
    }

    /// Met à jour graph_view à partir de la sélection legacy.
    /// Recharge également les fichiers du commit sélectionné.
    pub fn sync_legacy_selection(&mut self) {
        self.graph_view.rows.select(self.selected_index);
        // Recharger les fichiers du commit sélectionné
        if let Some(row) = self.graph.get(self.selected_index) {
            self.commit_files = self.repo.commit_diff(row.node.oid).unwrap_or_default();
            // Réinitialiser la sélection de fichier si nécessaire
            if self.file_selected_index >= self.commit_files.len() {
                self.file_selected_index = 0;
            }
            // Charger le diff du fichier sélectionné
            crate::handler::navigation::load_commit_file_diff(self);
        } else {
            self.commit_files.clear();
            self.file_selected_index = 0;
            self.selected_file_diff = None;
        }
    }
}

// Compatibilité: types exportés depuis l'ancien state.rs
pub use action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
