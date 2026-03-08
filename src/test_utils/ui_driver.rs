//! Helpers de tests d'integration pour piloter input -> dispatcher -> state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use std::path::Path;
use tempfile::TempDir;

use crate::git::repo::GitRepo;
use crate::handler::dispatcher::ActionDispatcher;
use crate::state::{AppAction, AppState};

pub struct UiTestHarness {
    _temp_dir: TempDir,
    pub state: AppState,
    dispatcher: ActionDispatcher,
}

impl UiTestHarness {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(temp_dir.path().to_string_lossy().as_ref()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
        state.screen_area = Rect::new(0, 0, 120, 40);

        Self {
            _temp_dir: temp_dir,
            state,
            dispatcher: ActionDispatcher::new(),
        }
    }

    pub fn dispatch(&mut self, action: AppAction) {
        self.dispatcher.dispatch(&mut self.state, action).unwrap();
    }

    pub fn send_key(&mut self, key: KeyEvent) {
        if let Some(action) = crate::ui::input::map_key_for_test(key, &self.state) {
            self.dispatch(action);
        }
    }

    pub fn send_char(&mut self, c: char) {
        self.send_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    pub fn send_enter(&mut self) {
        self.send_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    pub fn send_tab(&mut self) {
        self.send_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }

    pub fn send_esc(&mut self) {
        self.send_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    }

    pub fn send_text(&mut self, text: &str) {
        for c in text.chars() {
            self.send_char(c);
        }
    }

    pub fn write_file(&self, path: &str, content: &str) {
        let full_path = Path::new(&self.state.repo_path).join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full_path, content).unwrap();
    }

    pub fn stage_file(&self, path: &str) {
        let mut index = self.state.repo.repo.index().unwrap();
        index.add_path(Path::new(path)).unwrap();
        index.write().unwrap();
    }

    pub fn commit_all(&self, message: &str) -> git2::Oid {
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = self.state.repo.repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = self.state.repo.repo.find_tree(tree_oid).unwrap();
        let parent = self
            .state
            .repo
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| self.state.repo.repo.find_commit(oid).ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        self.state
            .repo
            .repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    pub fn commit_file(&self, path: &str, content: &str, message: &str) -> git2::Oid {
        self.write_file(path, content);
        self.stage_file(path);
        self.commit_all(message)
    }

    pub fn refresh_graph(&mut self) {
        let graph = self.state.repo.build_graph(50).unwrap();
        self.state.replace_graph(graph);
        self.state.refresh_commit_files();
        if !self.state.graph_view.commit_files.is_empty() {
            crate::handler::navigation::load_commit_file_diff(&mut self.state);
        }
    }

    pub fn refresh_staging(&mut self) {
        crate::handler::staging::refresh_staging(&mut self.state).unwrap();
    }
}
