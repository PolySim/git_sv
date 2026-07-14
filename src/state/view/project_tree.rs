//! État de la vue arborescence et historique par chemin.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

use crate::git::commit::CommitInfo;
use crate::git::diff::{DiffFile, DiffViewMode, FileDiff};
use crate::git::project_tree::{PathHistoryComparison, PathHistorySide};
use crate::state::project_search::fuzzy_path_score;
use crate::state::selection::ListSelection;
use crate::state::TextEditHistory;

/// Type d'une entrée visible dans l'arborescence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEntryKind {
    Directory,
    File,
}

/// Entrée aplatie de l'arborescence, prête à être affichée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTreeEntry {
    pub name: String,
    pub path: String,
    pub depth: usize,
    pub kind: ProjectEntryKind,
    pub expanded: bool,
}

impl ProjectTreeEntry {
    pub fn is_directory(&self) -> bool {
        self.kind == ProjectEntryKind::Directory
    }
}

/// Panneau actif de la vue arborescence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectTreeFocus {
    #[default]
    Tree,
    History,
    ChangedFiles,
    Diff,
}

/// Contexte de comparaison du chemin sélectionné avec une branche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTreeComparison {
    pub base_branch: String,
    pub target_branch: String,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}

/// État de la recherche rapide de fichiers et dossiers.
#[derive(Debug, Clone)]
pub struct ProjectTreeSearchState {
    pub is_active: bool,
    pub query: String,
    pub cursor: usize,
    pub selection_anchor: Option<usize>,
    pub edit_history: TextEditHistory,
    pub results: ListSelection<ProjectTreeEntry>,
}

impl Default for ProjectTreeSearchState {
    fn default() -> Self {
        Self {
            is_active: false,
            query: String::new(),
            cursor: 0,
            selection_anchor: None,
            edit_history: TextEditHistory::default(),
            results: ListSelection::new(),
        }
    }
}

/// État complet de la vue arborescence.
#[derive(Debug, Clone)]
pub struct ProjectTreeState {
    pub focus: ProjectTreeFocus,
    pub entries: ListSelection<ProjectTreeEntry>,
    pub history: ListSelection<CommitInfo>,
    pub history_loaded: bool,
    pub comparison: Option<ProjectTreeComparison>,
    history_sides: HashMap<git2::Oid, PathHistorySide>,
    pub changed_files: ListSelection<DiffFile>,
    pub commit_details_loaded: bool,
    preferred_changed_files_count: usize,
    pub selected_diff: Option<FileDiff>,
    pub diff_loaded: bool,
    pub diff_scroll_offset: usize,
    pub diff_horizontal_offset: usize,
    pub diff_total_lines: usize,
    pub diff_view_mode: DiffViewMode,
    pub search: ProjectTreeSearchState,
    file_paths: Vec<String>,
    all_entries: Vec<ProjectTreeEntry>,
    expanded_directories: BTreeSet<String>,
    known_directories: BTreeSet<String>,
}

impl Default for ProjectTreeState {
    fn default() -> Self {
        Self {
            focus: ProjectTreeFocus::Tree,
            entries: ListSelection::new(),
            history: ListSelection::new(),
            history_loaded: false,
            comparison: None,
            history_sides: HashMap::new(),
            changed_files: ListSelection::new(),
            commit_details_loaded: false,
            preferred_changed_files_count: 0,
            selected_diff: None,
            diff_loaded: false,
            diff_scroll_offset: 0,
            diff_horizontal_offset: 0,
            diff_total_lines: 0,
            diff_view_mode: DiffViewMode::Unified,
            search: ProjectTreeSearchState::default(),
            file_paths: Vec::new(),
            all_entries: Vec::new(),
            expanded_directories: BTreeSet::new(),
            known_directories: BTreeSet::new(),
        }
    }
}

impl ProjectTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remplace les fichiers et reconstruit les entrées visibles.
    pub fn set_files(&mut self, file_paths: Vec<String>) {
        let selected_path = self.selected_entry().map(|entry| entry.path.clone());
        self.file_paths = file_paths;

        let root = build_tree(&self.file_paths);
        let directories = collect_directories(&root);
        self.expanded_directories
            .retain(|path| directories.contains(path));
        self.known_directories = directories;

        self.all_entries.clear();
        flatten_tree(&root, 0, &self.known_directories, &mut self.all_entries);

        self.rebuild_visible_entries(selected_path.as_deref());
        self.update_search_results();
        self.invalidate_path_history();
    }

    pub fn selected_entry(&self) -> Option<&ProjectTreeEntry> {
        self.entries.selected_item()
    }

    /// Ouvre ou ferme le dossier sélectionné.
    pub fn toggle_selected_directory(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if !entry.is_directory() {
            return;
        }

        if !self.expanded_directories.remove(&entry.path) {
            self.expanded_directories.insert(entry.path.clone());
        }
        self.rebuild_visible_entries(Some(&entry.path));
    }

    /// Ouvre le dossier sélectionné.
    pub fn expand_selected_directory(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if entry.is_directory() && self.expanded_directories.insert(entry.path.clone()) {
            self.rebuild_visible_entries(Some(&entry.path));
        }
    }

    /// Ferme le dossier sélectionné ou remonte à son dossier parent.
    pub fn collapse_selected_or_select_parent(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if entry.is_directory() && self.expanded_directories.remove(&entry.path) {
            self.rebuild_visible_entries(Some(&entry.path));
            return;
        }

        let Some((parent, _)) = entry.path.rsplit_once('/') else {
            return;
        };
        if let Some(index) = self
            .entries
            .items()
            .iter()
            .position(|candidate| candidate.path == parent)
        {
            self.select_tree_entry(index);
        }
    }

    /// Sélectionne une entrée et bascule son dossier éventuel.
    pub fn activate_entry(&mut self, index: usize) {
        self.select_tree_entry(index);
        self.toggle_selected_directory();
    }

    /// Sélectionne une entrée et invalide les données du chemin précédent.
    pub fn select_tree_entry(&mut self, index: usize) {
        let previous = self.entries.selected_index();
        self.entries.select(index);
        if self.entries.selected_index() != previous {
            self.invalidate_path_history();
        }
    }

    pub fn selected_history_commit(&self) -> Option<&CommitInfo> {
        self.history.selected_item()
    }

    pub fn selected_changed_file(&self) -> Option<&DiffFile> {
        self.changed_files.selected_item()
    }

    /// Sélectionne un commit et invalide ses détails précédemment chargés.
    pub fn select_history_entry(&mut self, index: usize) {
        let previous = self.history.selected_index();
        self.history.select(index);
        if self.history.selected_index() != previous {
            self.invalidate_commit_details();
        }
    }

    /// Sélectionne un fichier et invalide le diff précédemment chargé.
    pub fn select_changed_file(&mut self, index: usize) {
        let previous = self.changed_files.selected_index();
        self.changed_files.select(index);
        if self.changed_files.selected_index() != previous {
            self.invalidate_diff();
        }
    }

    /// Remplace l'historique chargé pour le chemin sélectionné.
    pub fn set_path_history(&mut self, history: Vec<CommitInfo>) {
        self.history_sides.clear();
        self.history.set_items(history);
        self.history.select_first();
        self.history_loaded = true;
        self.invalidate_commit_details();
    }

    /// Active une comparaison de chemin avec une branche.
    pub fn start_comparison(&mut self, base_branch: String, target_branch: String) {
        self.comparison = Some(ProjectTreeComparison {
            base_branch,
            target_branch,
            ahead: None,
            behind: None,
        });
        self.invalidate_path_history();
    }

    /// Ferme la comparaison de chemin active.
    pub fn clear_comparison(&mut self) {
        self.comparison = None;
        self.invalidate_path_history();
    }

    /// Remplace l'historique par les commits divergents annotés par branche.
    pub fn set_compared_path_history(&mut self, comparison: PathHistoryComparison) {
        self.history_sides = comparison
            .commits
            .iter()
            .map(|entry| (entry.commit.oid, entry.side))
            .collect();
        self.history.set_items(
            comparison
                .commits
                .into_iter()
                .map(|entry| entry.commit)
                .collect(),
        );
        self.history.select_first();
        self.history_loaded = true;
        if let Some(active) = self.comparison.as_mut() {
            active.ahead = Some(comparison.ahead);
            active.behind = Some(comparison.behind);
        }
        self.invalidate_commit_details();
    }

    pub fn history_side(&self, oid: git2::Oid) -> Option<PathHistorySide> {
        self.history_sides.get(&oid).copied()
    }

    /// Regroupe en tête les fichiers correspondant au chemin consulté.
    pub fn set_changed_files(&mut self, files: Vec<DiffFile>) {
        let selected = self
            .selected_entry()
            .map(|entry| (entry.path.clone(), entry.is_directory()));
        let (mut preferred, others): (Vec<_>, Vec<_>) = files.into_iter().partition(|file| {
            selected
                .as_ref()
                .is_some_and(|(path, is_directory)| file_matches_path(file, path, *is_directory))
        });
        self.preferred_changed_files_count = preferred.len();
        preferred.extend(others);
        self.changed_files.set_items(preferred);
        self.changed_files.select_first();
        self.commit_details_loaded = true;
        self.invalidate_diff();
    }

    pub fn has_changed_files_separator(&self) -> bool {
        self.preferred_changed_files_count > 0
            && self.preferred_changed_files_count < self.changed_files.len()
    }

    pub fn changed_files_separator_index(&self) -> Option<usize> {
        self.has_changed_files_separator()
            .then_some(self.preferred_changed_files_count)
    }

    /// Traduit une ligne visuelle du panneau vers l'index du fichier.
    pub fn changed_file_index_at_visual_row(&self, row: usize) -> Option<usize> {
        let separator_index = self.changed_files_separator_index();
        let mut current_row = 0;
        for index in self.changed_files.scroll_offset()..self.changed_files.len() {
            if separator_index == Some(index) {
                if row == current_row {
                    return None;
                }
                current_row += 1;
            }
            if row == current_row {
                return Some(index);
            }
            current_row += 1;
        }
        None
    }

    pub fn invalidate_path_history(&mut self) {
        self.history.clear();
        self.history_sides.clear();
        self.history_loaded = false;
        if let Some(comparison) = self.comparison.as_mut() {
            comparison.ahead = None;
            comparison.behind = None;
        }
        self.invalidate_commit_details();
    }

    pub fn invalidate_commit_details(&mut self) {
        self.changed_files.clear();
        self.commit_details_loaded = false;
        self.preferred_changed_files_count = 0;
        self.invalidate_diff();
    }

    pub fn invalidate_diff(&mut self) {
        self.selected_diff = None;
        self.diff_loaded = false;
        self.diff_scroll_offset = 0;
        self.diff_horizontal_offset = 0;
        self.diff_total_lines = 0;
    }

    pub fn set_selected_diff(&mut self, diff: Option<FileDiff>) {
        self.selected_diff = diff;
        self.diff_loaded = true;
        self.diff_scroll_offset = 0;
        self.diff_horizontal_offset = 0;
        self.diff_total_lines = 0;
    }

    pub fn open_search(&mut self) {
        self.search.is_active = true;
        self.search.query.clear();
        self.search.cursor = 0;
        self.search.selection_anchor = None;
        self.search.edit_history.clear();
        self.update_search_results();
    }

    pub fn close_search(&mut self) {
        self.search.is_active = false;
    }

    pub fn update_search_results(&mut self) {
        if self.search.query.trim().is_empty() {
            self.search
                .results
                .set_items(self.all_entries.iter().take(100).cloned().collect());
            self.search.results.select_first();
            return;
        }

        let mut matches: Vec<_> = self
            .all_entries
            .iter()
            .filter_map(|entry| {
                fuzzy_path_score(&self.search.query, &entry.path)
                    .map(|score| (score, entry.clone()))
            })
            .collect();
        matches.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.path.cmp(&right.path))
        });
        self.search.results.set_items(
            matches
                .into_iter()
                .take(100)
                .map(|(_, entry)| entry)
                .collect(),
        );
        self.search.results.select_first();
    }

    /// Révèle un résultat de recherche dans l'arborescence et le sélectionne.
    pub fn reveal_path(&mut self, path: &str) {
        let previous_path = self.selected_entry().map(|entry| entry.path.clone());
        let mut current = String::new();
        let components: Vec<_> = path.split('/').collect();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(component);
            self.expanded_directories.insert(current.clone());
        }
        self.rebuild_visible_entries(Some(path));
        if self.selected_entry().map(|entry| &entry.path) != previous_path.as_ref() {
            self.invalidate_path_history();
        }
    }

    fn rebuild_visible_entries(&mut self, selected_path: Option<&str>) {
        let root = build_tree(&self.file_paths);
        let mut entries = Vec::new();
        flatten_tree(&root, 0, &self.expanded_directories, &mut entries);
        self.entries.set_items(entries);

        if let Some(selected_path) = selected_path {
            if let Some(index) = self
                .entries
                .items()
                .iter()
                .position(|entry| entry.path == selected_path)
            {
                self.entries.select(index);
            }
        }
    }
}

fn file_matches_path(file: &DiffFile, selected_path: &str, is_directory: bool) -> bool {
    let matches = |candidate: &str| {
        if is_directory {
            Path::new(candidate).starts_with(selected_path)
        } else {
            candidate == selected_path
        }
    };
    matches(&file.path) || file.old_path.as_deref().is_some_and(matches)
}

#[derive(Default)]
struct TreeNode {
    name: String,
    path: String,
    is_file: bool,
    children: BTreeMap<String, TreeNode>,
}

fn build_tree(paths: &[String]) -> TreeNode {
    let mut root = TreeNode::default();

    for path in paths {
        let components: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
        let mut node = &mut root;
        let mut current_path = String::new();

        for (index, component) in components.iter().enumerate() {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(component);

            node = node
                .children
                .entry((*component).to_string())
                .or_insert_with(|| TreeNode {
                    name: (*component).to_string(),
                    path: current_path.clone(),
                    ..TreeNode::default()
                });
            node.is_file = index + 1 == components.len();
        }
    }

    root
}

fn collect_directories(root: &TreeNode) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    collect_directories_recursive(root, &mut directories);
    directories
}

fn collect_directories_recursive(node: &TreeNode, directories: &mut BTreeSet<String>) {
    for child in node.children.values() {
        if !child.is_file {
            directories.insert(child.path.clone());
            collect_directories_recursive(child, directories);
        }
    }
}

fn flatten_tree(
    node: &TreeNode,
    depth: usize,
    expanded_directories: &BTreeSet<String>,
    entries: &mut Vec<ProjectTreeEntry>,
) {
    for child in node.children.values().filter(|child| !child.is_file) {
        let expanded = expanded_directories.contains(&child.path);
        entries.push(ProjectTreeEntry {
            name: child.name.clone(),
            path: child.path.clone(),
            depth,
            kind: ProjectEntryKind::Directory,
            expanded,
        });
        if expanded {
            flatten_tree(child, depth + 1, expanded_directories, entries);
        }
    }

    for child in node.children.values().filter(|child| child.is_file) {
        entries.push(ProjectTreeEntry {
            name: child.name.clone(),
            path: child.path.clone(),
            depth,
            kind: ProjectEntryKind::File,
            expanded: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_start_collapsed() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec![
            "README.md".to_string(),
            "src/lib.rs".to_string(),
            "src/ui/mod.rs".to_string(),
        ]);

        let paths: Vec<_> = state
            .entries
            .items()
            .iter()
            .map(|entry| (entry.path.as_str(), entry.depth, entry.kind))
            .collect();

        assert_eq!(
            paths,
            vec![
                ("src", 0, ProjectEntryKind::Directory),
                ("README.md", 0, ProjectEntryKind::File),
            ]
        );
        assert!(!state.selected_entry().unwrap().expanded);
    }

    #[test]
    fn toggling_directory_shows_descendants_and_keeps_selection() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["src/lib.rs".to_string(), "README.md".to_string()]);

        state.toggle_selected_directory();

        assert_eq!(state.entries.len(), 3);
        assert_eq!(state.selected_entry().unwrap().path, "src");
        assert!(state.selected_entry().unwrap().expanded);
    }

    #[test]
    fn collapsing_a_file_selects_its_parent_then_closes_it() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["src/ui/mod.rs".to_string()]);
        state.expand_selected_directory();
        state.entries.select(1);
        state.expand_selected_directory();
        state.entries.select(2);

        state.collapse_selected_or_select_parent();
        assert_eq!(state.selected_entry().unwrap().path, "src/ui");

        state.collapse_selected_or_select_parent();
        assert_eq!(state.selected_entry().unwrap().path, "src/ui");
        assert!(!state.selected_entry().unwrap().expanded);
        assert_eq!(state.entries.len(), 2);
    }

    #[test]
    fn quick_search_finds_typo_and_reveals_result() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec![
            "src/ui/project_tree_view.rs".to_string(),
            "README.md".to_string(),
        ]);
        assert!(!state
            .entries
            .items()
            .iter()
            .any(|entry| entry.path.ends_with("project_tree_view.rs")));

        state.open_search();
        state.search.query = "projet_tree_view".to_string();
        state.update_search_results();
        let result = state.search.results.selected_item().unwrap().path.clone();
        assert_eq!(result, "src/ui/project_tree_view.rs");

        state.reveal_path(&result);
        assert_eq!(state.selected_entry().unwrap().path, result);
    }

    #[test]
    fn selected_file_is_prioritized_in_changed_files() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["src/lib.rs".to_string(), "src/ui/mod.rs".to_string()]);
        state.reveal_path("src/ui/mod.rs");

        state.set_changed_files(vec![
            diff_file("README.md"),
            diff_file("src/ui/mod.rs"),
            diff_file("src/lib.rs"),
        ]);

        let paths: Vec<_> = state
            .changed_files
            .items()
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, vec!["src/ui/mod.rs", "README.md", "src/lib.rs"]);
        assert_eq!(state.changed_files_separator_index(), Some(1));
    }

    #[test]
    fn selected_directory_prioritizes_all_descendants() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["src/lib.rs".to_string(), "src/ui/mod.rs".to_string()]);

        state.set_changed_files(vec![
            diff_file("README.md"),
            diff_file("src/ui/mod.rs"),
            diff_file("src/lib.rs"),
        ]);

        let paths: Vec<_> = state
            .changed_files
            .items()
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, vec!["src/ui/mod.rs", "src/lib.rs", "README.md"]);
        assert_eq!(state.changed_files_separator_index(), Some(2));
        assert_eq!(state.changed_file_index_at_visual_row(2), None);
        assert_eq!(state.changed_file_index_at_visual_row(3), Some(2));
    }

    #[test]
    fn changing_tree_selection_invalidates_all_dependent_panels() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["a.rs".to_string(), "b.rs".to_string()]);
        state.set_path_history(vec![commit_info(1)]);
        state.set_changed_files(vec![diff_file("a.rs")]);
        state.set_selected_diff(None);

        state.select_tree_entry(1);

        assert_eq!(state.selected_entry().unwrap().path, "b.rs");
        assert!(state.history.is_empty());
        assert!(!state.history_loaded);
        assert!(state.changed_files.is_empty());
        assert!(!state.commit_details_loaded);
        assert!(!state.diff_loaded);
    }

    #[test]
    fn selecting_same_tree_entry_keeps_loaded_panels() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["a.rs".to_string(), "b.rs".to_string()]);
        state.set_path_history(vec![commit_info(1)]);
        state.set_changed_files(vec![diff_file("a.rs")]);
        state.set_selected_diff(None);

        state.select_tree_entry(0);

        assert!(state.history_loaded);
        assert!(state.commit_details_loaded);
        assert!(state.diff_loaded);
    }

    #[test]
    fn changing_history_and_file_selection_invalidates_only_descendants() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["a.rs".to_string()]);
        state.set_path_history(vec![commit_info(1), commit_info(2)]);
        state.set_changed_files(vec![diff_file("a.rs"), diff_file("b.rs")]);
        state.set_selected_diff(None);

        state.select_history_entry(1);

        assert!(state.history_loaded);
        assert!(!state.commit_details_loaded);
        assert!(!state.diff_loaded);

        state.set_changed_files(vec![diff_file("a.rs"), diff_file("b.rs")]);
        state.set_selected_diff(None);
        state.select_changed_file(1);

        assert!(state.commit_details_loaded);
        assert!(!state.diff_loaded);
    }

    #[test]
    fn compared_history_tracks_sides_and_survives_path_changes() {
        let mut state = ProjectTreeState::new();
        state.set_files(vec!["a.rs".to_string(), "b.rs".to_string()]);
        state.start_comparison("main".to_string(), "feature".to_string());
        let current = commit_info(1);
        let target = commit_info(2);
        state.set_compared_path_history(PathHistoryComparison {
            commits: vec![
                crate::git::project_tree::ComparedPathCommit {
                    commit: current.clone(),
                    side: PathHistorySide::Current,
                },
                crate::git::project_tree::ComparedPathCommit {
                    commit: target.clone(),
                    side: PathHistorySide::Target,
                },
            ],
            ahead: 1,
            behind: 1,
        });

        assert_eq!(
            state.history_side(current.oid),
            Some(PathHistorySide::Current)
        );
        assert_eq!(
            state.history_side(target.oid),
            Some(PathHistorySide::Target)
        );
        assert_eq!(
            state
                .comparison
                .as_ref()
                .map(|comparison| (comparison.ahead, comparison.behind)),
            Some((Some(1), Some(1)))
        );

        state.select_tree_entry(1);

        let comparison = state.comparison.as_ref().unwrap();
        assert_eq!(comparison.target_branch, "feature");
        assert_eq!((comparison.ahead, comparison.behind), (None, None));
        assert!(!state.history_loaded);
        assert_eq!(state.history_side(current.oid), None);

        state.clear_comparison();
        assert!(state.comparison.is_none());
    }

    fn commit_info(byte: u8) -> CommitInfo {
        CommitInfo {
            oid: git2::Oid::from_bytes(&[byte; 20]).unwrap(),
            message: format!("commit {byte}"),
            author: "Test".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 0,
            parents: Vec::new(),
            changed_paths: None,
        }
    }

    fn diff_file(path: &str) -> DiffFile {
        DiffFile {
            path: path.to_string(),
            status: crate::git::diff::DiffStatus::Modified,
            old_path: None,
            additions: 1,
            deletions: 1,
        }
    }
}
