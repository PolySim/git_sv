pub mod blame;
pub mod branch;
pub mod commit;
pub mod conflict;
pub mod diff;
pub mod discard;
pub mod graph;
pub mod merge;
pub mod remote;
pub mod repo;
pub mod search;
pub mod stash;
pub mod worktree;

pub use repo::GitRepo;

#[cfg(test)]
pub mod tests {
    pub mod test_utils;
}
