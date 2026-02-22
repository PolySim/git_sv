//! Mock repository pour les tests sans filesystem.

/// Mock d'un repository git pour les tests.
#[derive(Default)]
pub struct MockRepo {
    pub branches: Vec<MockBranch>,
    pub commits: Vec<MockCommit>,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub current_branch: Option<String>,
}

pub struct MockBranch {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
}

pub struct MockCommit {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub parent_oids: Vec<String>,
}

impl MockRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_branch(mut self, name: &str, is_head: bool) -> Self {
        self.branches.push(MockBranch {
            name: name.to_string(),
            is_head,
            is_remote: false,
        });
        if is_head {
            self.current_branch = Some(name.to_string());
        }
        self
    }

    pub fn with_commit(mut self, oid: &str, message: &str) -> Self {
        self.commits.push(MockCommit {
            oid: oid.to_string(),
            message: message.to_string(),
            author: "Test Author".to_string(),
            parent_oids: vec![],
        });
        self
    }

    pub fn with_staged(mut self, file: &str) -> Self {
        self.staged_files.push(file.to_string());
        self
    }

    pub fn with_unstaged(mut self, file: &str) -> Self {
        self.unstaged_files.push(file.to_string());
        self
    }
}

/// Trait pour permettre le mocking dans les handlers.
pub trait RepositoryLike {
    fn current_branch(&self) -> Option<&str>;
    fn list_branches(&self) -> Vec<String>;
    fn staged_files(&self) -> &[String];
    fn unstaged_files(&self) -> &[String];
}

impl RepositoryLike for MockRepo {
    fn current_branch(&self) -> Option<&str> {
        self.current_branch.as_deref()
    }

    fn list_branches(&self) -> Vec<String> {
        self.branches.iter().map(|b| b.name.clone()).collect()
    }

    fn staged_files(&self) -> &[String] {
        &self.staged_files
    }

    fn unstaged_files(&self) -> &[String] {
        &self.unstaged_files
    }
}
