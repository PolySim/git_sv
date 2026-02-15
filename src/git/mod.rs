pub mod branch;
pub mod commit;
pub mod diff;
pub mod graph;
pub mod merge;
pub mod repo;
pub mod stash;
pub mod worktree;

pub use repo::GitRepo;

#[cfg(test)]
pub mod tests {
    pub mod test_utils;
}
