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
            } else if reference.is_remote() || name.contains('/') {
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

    Ok(map)
}
