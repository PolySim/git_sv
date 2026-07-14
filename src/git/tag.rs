//! Gestion des tags Git.

use git2::{Oid, Repository};

use crate::error::Result;

/// Liste les noms de tags triés.
pub fn list_tags(repo: &Repository) -> Result<Vec<String>> {
    let names = repo.tag_names(None)?;
    let mut tags = names
        .iter()
        .flatten()
        .map(str::to_string)
        .collect::<Vec<_>>();
    tags.sort();
    Ok(tags)
}

/// Crée un tag léger sur le commit donné.
pub fn create_tag(repo: &Repository, name: &str, target: Oid) -> Result<()> {
    let commit = repo.find_commit(target)?;
    repo.tag_lightweight(name, commit.as_object(), false)?;
    Ok(())
}

/// Supprime un tag.
pub fn delete_tag(repo: &Repository, name: &str) -> Result<()> {
    repo.tag_delete(name)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn test_create_list_and_delete_tag() {
        let (_directory, repo) = create_test_repo();
        let oid = commit_file(&repo, "file.txt", "content", "commit");

        create_tag(&repo, "v1.0.0", oid).unwrap();
        assert_eq!(list_tags(&repo).unwrap(), vec!["v1.0.0"]);

        delete_tag(&repo, "v1.0.0").unwrap();
        assert!(list_tags(&repo).unwrap().is_empty());
    }
}
