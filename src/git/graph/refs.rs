use std::collections::HashMap;

use git2::{Oid, Repository};

use crate::error::Result;

use super::{RefInfo, RefType};

pub(super) fn collect_refs(repo: &Repository) -> Result<HashMap<Oid, Vec<RefInfo>>> {
    let mut map: HashMap<Oid, Vec<RefInfo>> = HashMap::new();

    let head_oid = repo.head().ok().and_then(|h| h.target());
    let head_branch = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            h.shorthand().map(|s| s.to_string())
        } else {
            None
        }
    });

    for reference in repo.references()? {
        let reference = reference?;
        if let Some(name) = reference.shorthand() {
            if name == "HEAD" {
                continue;
            }

            let ref_type = if reference.is_tag() {
                RefType::Tag
            } else if reference.is_remote() {
                RefType::RemoteBranch
            } else {
                RefType::LocalBranch
            };

            let target_oid = if reference.is_tag() {
                reference
                    .peel(git2::ObjectType::Commit)
                    .ok()
                    .and_then(|obj| obj.as_commit().map(|c| c.id()))
            } else {
                reference.target()
            };

            if let Some(oid) = target_oid {
                map.entry(oid).or_default().push(RefInfo {
                    name: name.to_string(),
                    ref_type,
                });
            }
        }
    }

    if let (Some(oid), Some(branch)) = (head_oid, head_branch) {
        map.entry(oid)
            .or_default()
            .push(RefInfo::new(branch, RefType::Head));
    }

    for refs in map.values_mut() {
        refs.sort_by(|left, right| {
            ref_priority(&left.ref_type)
                .cmp(&ref_priority(&right.ref_type))
                .then_with(|| left.name.cmp(&right.name))
        });
    }

    Ok(map)
}

fn ref_priority(ref_type: &RefType) -> u8 {
    match ref_type {
        RefType::Head => 0,
        RefType::LocalBranch => 1,
        RefType::Tag => 2,
        RefType::RemoteBranch => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_collect_refs_distinguishes_local_branch_with_slash_from_remote() {
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        let commit = repo.find_commit(commit_oid).unwrap();
        repo.branch("feature/ui", &commit, false).unwrap();
        repo.reference("refs/remotes/origin/main", commit_oid, false, "test")
            .unwrap();

        let refs = collect_refs(&repo).unwrap();
        let commit_refs = refs.get(&commit_oid).unwrap();

        assert!(commit_refs.iter().any(|reference| {
            reference.name == "feature/ui" && reference.ref_type == RefType::LocalBranch
        }));
        assert!(commit_refs.iter().any(|reference| {
            reference.name == "origin/main" && reference.ref_type == RefType::RemoteBranch
        }));
        assert_eq!(
            commit_refs.first().map(|reference| &reference.ref_type),
            Some(&RefType::Head)
        );
    }
}
