//! Couche d'accès au repository git (libgit2).
//!
//! Ce module expose tous les sous-modules git et les types
//! principaux nécessaires aux autres modules de l'application.

pub mod blame;
pub mod branch;
pub mod commit;
pub mod conflict;
pub mod diff;
pub mod discard;
pub mod graph;
pub mod helpers;
pub mod merge;
pub mod project_tree;
pub mod rebase;
pub mod remote;
pub mod repo;
pub mod search;
pub mod staging;
pub mod stash;
pub mod worktree;

pub use repo::GitRepo;

#[cfg(test)]
pub mod tests {
    pub mod test_utils;
}
