//! Comparaison directe des arbres de deux commits, sans consulter leurs parents.

use std::{borrow::Cow, collections::HashMap};

use git2::{Delta, Diff, DiffOptions, FileMode, Odb, OdbObject, Oid, Patch, Repository};

use crate::error::Result;

use super::{
    blob_image_preview, bounded_diff_options, diff_status, extract_diff_lines, image_format,
    limited_file_diff, DiffStatus, FileDiff, DIFF_UNAVAILABLE_MESSAGE, MAX_DIFF_INPUT_BYTES,
};

/// Delta autonome : la sélection ne dépend plus des arbres ni des références.
#[derive(Debug, Clone)]
pub struct ComparisonDelta {
    pub status: DiffStatus,
    old: Option<SnapshotFile>,
    new: Option<SnapshotFile>,
}

#[derive(Debug, Clone)]
struct SnapshotFile {
    id: Oid,
    path: Vec<u8>,
    mode: FileMode,
}

/// Diff sélectionné et notices qui restent visibles même avec un aperçu d'image.
#[derive(Debug)]
pub struct ComparisonDetail {
    pub diff: FileDiff,
    pub metadata: Vec<String>,
}

/// Liste les différences de A (ancien) vers B (nouveau), par OIDs de commits exacts.
///
/// Les renommages sont détectés à contenu et mode identiques, uniquement quand
/// le couple suppression/ajout est unique. Sinon les deltas restent séparés.
/// Aucun blob ni patch n'est chargé pour construire cette liste.
pub fn compare_commit_files(repo: &Repository, a: Oid, b: Oid) -> Result<Vec<ComparisonDelta>> {
    let old = repo.find_commit(a)?.tree()?;
    let new = repo.find_commit(b)?.tree()?;
    let mut options = DiffOptions::new();
    // Un changement de type entre fichiers, liens et gitlinks reste un seul delta.
    options.include_typechange(true);
    let diff = repo.diff_tree_to_tree(Some(&old), Some(&new), Some(&mut options))?;
    snapshot_changes(&diff).collect()
}

/// Ne charge que le contenu des deux côtés du delta sélectionné.
pub fn compare_commit_file(repo: &Repository, file: &ComparisonDelta) -> Result<ComparisonDetail> {
    let extracted = file.with_patch(&repo.odb()?, extract_diff_lines)?;
    let patch_unavailable = extracted.is_none();
    let path = file.path();
    let mut diff = match extracted {
        Some((lines, additions, deletions)) => FileDiff {
            path: path.to_string(),
            status: file.status.clone(),
            lines,
            additions,
            deletions,
            image_preview: None,
        },
        None => limited_file_diff(&path, file.status.clone(), DIFF_UNAVAILABLE_MESSAGE),
    };
    let display_file = file.display_file();
    if matches!(
        display_file.mode,
        FileMode::Blob | FileMode::BlobExecutable | FileMode::BlobGroupWritable
    ) {
        diff.image_preview = image_format(&path)
            .and_then(|format| blob_image_preview(repo, display_file.id, format));
    }
    let metadata =
        file.metadata(patch_unavailable || diff.is_truncated() || diff.image_preview.is_some());
    Ok(ComparisonDetail { diff, metadata })
}

fn exact_key(file: &git2::DiffFile<'_>) -> (Oid, i32) {
    (file.id(), file.mode().into())
}

fn snapshot_changes<'a>(diff: &'a Diff<'_>) -> impl Iterator<Item = Result<ComparisonDelta>> + 'a {
    let mut added = HashMap::new();
    let mut deleted = HashMap::new();
    // Appariement linéaire en nombre de deltas, sans lire les blobs ni comparer
    // toutes les paires. Une clé dupliquée ne désigne pas un renommage certain.
    for (idx, delta) in diff.deltas().enumerate() {
        let (candidates, file) = match delta.status() {
            Delta::Added => (&mut added, delta.new_file()),
            Delta::Deleted => (&mut deleted, delta.old_file()),
            _ => continue,
        };
        candidates
            .entry(exact_key(&file))
            .and_modify(|candidate| *candidate = None)
            .or_insert(Some(idx));
    }
    deleted.retain(|key, source| source.is_some() && matches!(added.get(key), Some(Some(_))));

    diff.deltas()
        .filter_map(move |delta| {
            let mut old = delta.old_file();
            let new = delta.new_file();
            let status = match delta.status() {
                Delta::Deleted if deleted.contains_key(&exact_key(&old)) => return None,
                Delta::Added => match deleted.get(&exact_key(&new)) {
                    Some(Some(source)) => {
                        old = diff.get_delta(*source)?.old_file();
                        DiffStatus::Renamed
                    }
                    _ => DiffStatus::Added,
                },
                Delta::Typechange => DiffStatus::Modified,
                other => diff_status(other)?,
            };
            Some((status, old, new))
        })
        .map(|(status, old, new)| {
            Ok(ComparisonDelta {
                status,
                old: SnapshotFile::from_delta(old)?,
                new: SnapshotFile::from_delta(new)?,
            })
        })
}

impl SnapshotFile {
    fn from_delta(file: git2::DiffFile<'_>) -> Result<Option<Self>> {
        if !file.exists() {
            return Ok(None);
        }
        let path = file
            .path_bytes()
            .ok_or_else(|| git2::Error::from_str("Chemin du diff indisponible"))?;
        Ok(Some(Self {
            id: file.id(),
            path: path.to_vec(),
            mode: file.mode(),
        }))
    }

    fn is_gitlink(&self) -> bool {
        self.mode == FileMode::Commit
    }
}

impl ComparisonDelta {
    fn display_file(&self) -> &SnapshotFile {
        self.new
            .as_ref()
            .or(self.old.as_ref())
            .expect("delta sans fichier")
    }

    /// Chemin d'affichage, sans utiliser la conversion UTF-8 comme identité.
    pub fn path(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.display_file().path)
    }

    /// Chemin source d'un renommage exact.
    pub fn old_path(&self) -> Option<Cow<'_, str>> {
        self.old
            .as_ref()
            .filter(|_| self.status == DiffStatus::Renamed)
            .map(|file| String::from_utf8_lossy(&file.path))
    }

    fn metadata(&self, patch_may_hide_gitlink: bool) -> Vec<String> {
        let mut lines = Vec::new();
        if let (Some(old), Some(new)) = (&self.old, &self.new) {
            if old.mode != new.mode {
                lines.push(format!(
                    "Mode {:06o} → {:06o}",
                    u32::from(old.mode),
                    u32::from(new.mode)
                ));
            }
        }
        if patch_may_hide_gitlink {
            let gitlink_id = |file: Option<&SnapshotFile>| {
                file.filter(|file| file.is_gitlink()).map(|file| file.id)
            };
            let old = gitlink_id(self.old.as_ref());
            let new = gitlink_id(self.new.as_ref());
            if old != new {
                lines.extend(old.map(|id| format!("- Subproject commit {id}")));
                lines.extend(new.map(|id| format!("+ Subproject commit {id}")));
            }
        }
        lines
    }

    fn with_patch<T: Default>(
        &self,
        odb: &Odb<'_>,
        extract: impl FnOnce(&Patch<'_>) -> T,
    ) -> Result<Option<T>> {
        // Un gitlink représente un OID, pas le contenu d'un objet du dépôt.
        if matches!((&self.old, &self.new), (Some(old), Some(new))
            if old.id == new.id && old.is_gitlink() == new.is_gitlink())
        {
            return Ok(Some(T::default()));
        }
        let Some(old) = SnapshotContent::read(odb, self.old.as_ref())? else {
            return Ok(None);
        };
        let Some(new) = SnapshotContent::read(odb, self.new.as_ref())? else {
            return Ok(None);
        };
        // Des buffers sans chemins conservent le rendu par octets (blobs et
        // gitlinks synthétiques), sans les attributs du worktree/index.
        let patch = Patch::from_buffers(
            old.bytes(),
            None,
            new.bytes(),
            None,
            Some(&mut bounded_diff_options()),
        )?;
        Ok((!patch.delta().flags().contains(git2::DiffFlags::BINARY)).then(|| extract(&patch)))
    }
}

enum SnapshotContent<'a> {
    Empty,
    Blob(OdbObject<'a>),
    Gitlink(String),
}

impl<'a> SnapshotContent<'a> {
    fn read(odb: &'a Odb<'_>, file: Option<&SnapshotFile>) -> Result<Option<Self>> {
        let Some(file) = file else {
            return Ok(Some(Self::Empty));
        };
        if file.is_gitlink() {
            return Ok(Some(Self::Gitlink(format!(
                "Subproject commit {}\n",
                file.id
            ))));
        }
        let (size, kind) = odb.read_header(file.id)?;
        if size > MAX_DIFF_INPUT_BYTES as usize || kind != git2::ObjectType::Blob {
            return Ok(None);
        }
        Ok(Some(Self::Blob(odb.read(file.id)?)))
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Self::Empty => b"",
            Self::Blob(blob) => blob.data(),
            Self::Gitlink(text) => text.as_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use git2::{FileMode, Oid, Repository};

    use super::{compare_commit_file, compare_commit_files, ComparisonDelta};
    use crate::git::diff::{
        DiffLineType, DiffStatus, DIFF_UNAVAILABLE_MESSAGE, MAX_DIFF_INPUT_BYTES, MAX_DIFF_LINES,
    };
    use crate::git::tests::test_utils::{commit_file, create_file, create_test_repo};

    fn snapshot(repo: &Repository, parent: Option<Oid>, files: &[(&str, &str)]) -> Oid {
        let entries: Vec<_> = files
            .iter()
            .map(|(path, content)| {
                (
                    path.as_bytes(),
                    repo.blob(content.as_bytes()).unwrap(),
                    FileMode::Blob,
                )
            })
            .collect();
        snapshot_entries(repo, parent, &entries)
    }

    fn snapshot_entries(
        repo: &Repository,
        parent: Option<Oid>,
        entries: &[(&[u8], Oid, FileMode)],
    ) -> Oid {
        let mut builder = repo.treebuilder(None).unwrap();
        for &(path, id, mode) in entries {
            builder.insert(path, id, mode.into()).unwrap();
        }
        let tree = repo.find_tree(builder.write().unwrap()).unwrap();
        let parent = parent.map(|id| repo.find_commit(id).unwrap());
        let parents: Vec<_> = parent.iter().collect();
        let sig = repo.signature().unwrap();
        repo.commit(None, &sig, &sig, "snapshot", &tree, &parents)
            .unwrap()
    }

    fn get<'a>(files: &'a [ComparisonDelta], path: &str) -> &'a ComparisonDelta {
        files.iter().find(|file| file.path() == path).unwrap()
    }

    #[test]
    fn divergent_branches_compare_the_tips_in_both_directions() {
        let (_temp, repo) = create_test_repo();
        let base = snapshot(&repo, None, &[("[a].txt", "base\n")]);
        let left = snapshot(
            &repo,
            Some(base),
            &[("[a].txt", "left\n"), ("left.txt", "only left\n")],
        );
        let right = snapshot(
            &repo,
            Some(base),
            &[("[a].txt", "right\n"), ("right.txt", "only right\n")],
        );
        repo.branch("left", &repo.find_commit(left).unwrap(), false)
            .unwrap();
        repo.branch("right", &repo.find_commit(right).unwrap(), false)
            .unwrap();

        for (a, b, old, new) in [
            (left, right, "left", "right"),
            (right, left, "right", "left"),
        ] {
            let files = compare_commit_files(&repo, a, b).unwrap();
            assert_eq!(files.len(), 3);
            let added = get(&files, &format!("{new}.txt"));
            let deleted = get(&files, &format!("{old}.txt"));
            assert_eq!(added.status, DiffStatus::Added);
            assert_eq!(deleted.status, DiffStatus::Deleted);
            for (file, line_type, content, counts) in [
                (added, DiffLineType::Addition, format!("only {new}"), (1, 0)),
                (
                    deleted,
                    DiffLineType::Deletion,
                    format!("only {old}"),
                    (0, 1),
                ),
            ] {
                let detail = compare_commit_file(&repo, file).unwrap().diff;
                assert_eq!((detail.additions, detail.deletions), counts);
                assert!(detail
                    .lines
                    .iter()
                    .any(|line| line.line_type == line_type && line.content == content));
            }
            let modified = get(&files, "[a].txt");
            assert_eq!(modified.status, DiffStatus::Modified);
            let detail = compare_commit_file(&repo, modified).unwrap().diff;
            assert_eq!((detail.additions, detail.deletions), (1, 1));
            assert!(detail
                .lines
                .iter()
                .any(|line| line.line_type == DiffLineType::Deletion
                    && line.content == old
                    && line.old_lineno == Some(1)));
            assert!(detail
                .lines
                .iter()
                .any(|line| line.line_type == DiffLineType::Addition
                    && line.content == new
                    && line.new_lineno == Some(1)));
        }
        assert!(compare_commit_files(&repo, left, left).unwrap().is_empty());
        let same_tree = snapshot(&repo, Some(base), &[("[a].txt", "base\n")]);
        assert!(compare_commit_files(&repo, base, same_tree)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn exact_rename_requires_the_path_pair_and_does_not_guess_duplicate_blobs() {
        let (_temp, repo) = create_test_repo();
        let a = snapshot(
            &repo,
            None,
            &[
                ("before.txt", "same\n"),
                ("dup1.txt", "dup\n"),
                ("dup2.txt", "dup\n"),
            ],
        );
        let b = snapshot(
            &repo,
            None,
            &[("after.txt", "same\n"), ("dup3.txt", "dup\n")],
        );
        for (a, b, from, to) in [
            (a, b, "before.txt", "after.txt"),
            (b, a, "after.txt", "before.txt"),
        ] {
            let files = compare_commit_files(&repo, a, b).unwrap();
            assert_eq!(files.len(), 4);
            let file = get(&files, to);
            assert_eq!(file.status, DiffStatus::Renamed);
            assert_eq!(file.old_path().as_deref(), Some(from));
            let (old, new) = (file.old.as_ref().unwrap(), file.new.as_ref().unwrap());
            assert_eq!(
                (old.path.as_slice(), new.path.as_slice()),
                (from.as_bytes(), to.as_bytes())
            );
            assert_eq!(old.id, new.id);
            assert_eq!(old.mode, new.mode);
            let detail = compare_commit_file(&repo, file).unwrap().diff;
            assert_eq!(detail.path, to);
            assert_eq!(detail.status, DiffStatus::Renamed);
            assert_eq!((detail.additions, detail.deletions), (0, 0));
            assert!(detail.lines.is_empty());
            assert!(files
                .iter()
                .filter(|file| file.path().starts_with("dup"))
                .all(
                    |file| matches!(file.status, DiffStatus::Added | DiffStatus::Deleted)
                        && file.old_path().is_none()
                ));
        }
    }

    #[test]
    fn comparison_does_not_read_dirty_attributes_or_modify_work_data() {
        let (_temp, repo) = create_test_repo();
        let a = commit_file(&repo, "test.txt", "base\n", "Base");
        let b = snapshot(&repo, Some(a), &[("test.txt", "target\n")]);
        create_file(&repo, "test.txt", "staged\n");
        create_file(&repo, ".gitattributes", "*.txt -diff\n");
        let mut index = repo.index().unwrap();
        for path in ["test.txt", ".gitattributes"] {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        create_file(&repo, "test.txt", "unstaged\n");
        create_file(&repo, "local.txt", "untracked\n");
        let workdir = repo.workdir().unwrap();
        let paths = [
            repo.path().join("HEAD"),
            repo.path().join("refs/heads/main"),
            repo.path().join("index"),
            workdir.join("test.txt"),
            workdir.join(".gitattributes"),
            workdir.join("local.txt"),
        ];
        let before = paths.each_ref().map(|path| fs::read(path).unwrap());

        let files = compare_commit_files(&repo, a, b).unwrap();
        assert_eq!(files.len(), 1);
        let detail = compare_commit_file(&repo, &files[0]).unwrap().diff;
        assert_eq!((detail.additions, detail.deletions), (1, 1));
        assert!(detail.lines.iter().any(|line| line.content == "target"));
        assert_eq!(before, paths.each_ref().map(|path| fs::read(path).unwrap()));
    }

    #[test]
    fn large_diffs_are_bounded_and_non_text_files_keep_their_status() {
        let (_temp, repo) = create_test_repo();
        let a = snapshot(&repo, None, &[]);
        let b = snapshot(
            &repo,
            Some(a),
            &[
                ("many.txt", &"line\n".repeat(MAX_DIFF_LINES + 3)),
                ("large.txt", &"x".repeat(MAX_DIFF_INPUT_BYTES as usize + 1)),
                ("binary.dat", "\0before\0"),
            ],
        );
        let files = compare_commit_files(&repo, a, b).unwrap();
        let many = get(&files, "many.txt");
        let detail = compare_commit_file(&repo, many).unwrap().diff;
        assert!(detail.is_truncated());
        assert_eq!(detail.additions, MAX_DIFF_LINES + 3);
        for path in ["large.txt", "binary.dat"] {
            let file = get(&files, path);
            let detail = compare_commit_file(&repo, file).unwrap().diff;
            assert_eq!(detail.status, DiffStatus::Added);
            assert_eq!((detail.additions, detail.deletions), (0, 0));
            assert_eq!(detail.lines.len(), 1);
            assert_eq!(detail.lines[0].content, DIFF_UNAVAILABLE_MESSAGE);
        }
    }

    #[test]
    fn type_change_is_one_file_at_the_same_path() {
        let (_temp, repo) = create_test_repo();
        let a = snapshot(&repo, None, &[("file", "regular\n")]);
        let old_commit = repo.find_commit(a).unwrap();
        let mut tree = repo.treebuilder(None).unwrap();
        tree.insert("file", repo.blob(b"target").unwrap(), FileMode::Link.into())
            .unwrap();
        let tree = repo.find_tree(tree.write().unwrap()).unwrap();
        let sig = repo.signature().unwrap();
        let b = repo
            .commit(None, &sig, &sig, "symlink", &tree, &[&old_commit])
            .unwrap();

        let files = compare_commit_files(&repo, a, b).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, DiffStatus::Modified);
        assert_eq!(files[0].old_path(), None);
        let detail = compare_commit_file(&repo, &files[0]).unwrap();
        assert_eq!(detail.metadata, ["Mode 100644 → 120000"]);
        assert!(detail
            .diff
            .lines
            .iter()
            .any(|line| line.line_type == DiffLineType::Addition && line.content == "target"));
    }

    #[test]
    fn listing_needs_no_blobs_and_selection_needs_no_commits_or_trees() {
        let (_temp, repo) = create_test_repo();
        let a = snapshot(&repo, None, &[("selected.txt", "before\n")]);
        let b = snapshot(
            &repo,
            Some(a),
            &[("selected.txt", "after\n"), ("missing.txt", "gone\n")],
        );
        let old_tree = repo.find_commit(a).unwrap().tree_id();
        let new_tree = repo.find_commit(b).unwrap().tree().unwrap();
        let missing_blob = new_tree.get_name("missing.txt").unwrap().id();
        let remove_object = |id: Oid| {
            let hex = id.to_string();
            let (dir, name) = hex.split_at(2);
            fs::remove_file(repo.path().join("objects").join(dir).join(name)).unwrap();
        };
        remove_object(missing_blob);
        let fresh = Repository::open(repo.path()).unwrap();
        assert!(fresh.find_blob(missing_blob).is_err());
        let files = compare_commit_files(&fresh, a, b).unwrap();
        assert_eq!(files.len(), 2);
        assert!(compare_commit_file(&fresh, get(&files, "missing.txt")).is_err());

        for id in [a, b, old_tree, new_tree.id()] {
            remove_object(id);
        }
        let fresh = Repository::open(repo.path()).unwrap();
        assert!(compare_commit_files(&fresh, a, b).is_err());
        let detail = compare_commit_file(&fresh, get(&files, "selected.txt"))
            .unwrap()
            .diff;
        assert_eq!((detail.additions, detail.deletions), (1, 1));
        assert!(detail.lines.iter().any(|line| line.content == "after"));
    }

    #[test]
    fn non_utf8_paths_are_kept_as_bytes_and_select_the_right_blobs() {
        let (_temp, repo) = create_test_repo();
        let moved = repo.blob(b"moved\n").unwrap();
        let a = snapshot_entries(&repo, None, &[(b"before-\xff", moved, FileMode::Blob)]);
        let b = snapshot_entries(
            &repo,
            Some(a),
            &[
                (b"after-\xfe", moved, FileMode::Blob),
                (b"file-\xfe", repo.blob(b"one\n").unwrap(), FileMode::Blob),
                (b"file-\xff", repo.blob(b"two\n").unwrap(), FileMode::Blob),
            ],
        );
        let files = compare_commit_files(&repo, a, b).unwrap();
        assert_eq!(files.len(), 3);
        let rename = files
            .iter()
            .find(|file| file.status == DiffStatus::Renamed)
            .unwrap();
        assert_eq!(rename.old.as_ref().unwrap().path, b"before-\xff");
        assert_eq!(rename.new.as_ref().unwrap().path, b"after-\xfe");
        assert!(compare_commit_file(&repo, rename)
            .unwrap()
            .diff
            .lines
            .is_empty());
        for (path, text) in [(b"file-\xfe", "one"), (b"file-\xff", "two")] {
            let file = files
                .iter()
                .find(|file| file.display_file().path == path)
                .unwrap();
            assert_eq!(file.path(), "file-�");
            let diff = compare_commit_file(&repo, file).unwrap().diff;
            assert!(diff.lines.iter().any(|line| line.content == text));
        }
    }

    #[test]
    fn gitlink_add_delete_and_update_need_no_local_submodule_objects() {
        let (_temp, repo) = create_test_repo();
        let old = Oid::from_str("1111111111111111111111111111111111111111").unwrap();
        let new = Oid::from_str("2222222222222222222222222222222222222222").unwrap();
        assert!(repo.find_object(old, None).is_err());
        assert!(repo.find_object(new, None).is_err());
        let a = snapshot_entries(&repo, None, &[(b"sub", old, FileMode::Commit)]);
        let b = snapshot_entries(&repo, Some(a), &[(b"sub", new, FileMode::Commit)]);
        let empty = snapshot(&repo, None, &[]);
        for (from, to, status, removed, added) in [
            (a, b, DiffStatus::Modified, Some(old), Some(new)),
            (empty, a, DiffStatus::Added, None, Some(old)),
            (b, empty, DiffStatus::Deleted, Some(new), None),
        ] {
            let files = compare_commit_files(&repo, from, to).unwrap();
            assert_eq!(files.len(), 1);
            let detail = compare_commit_file(&repo, &files[0]).unwrap();
            assert_eq!(detail.diff.status, status);
            assert_eq!(
                (detail.diff.additions, detail.diff.deletions),
                (usize::from(added.is_some()), usize::from(removed.is_some()))
            );
            for (line_type, id) in [
                (DiffLineType::Deletion, removed),
                (DiffLineType::Addition, added),
            ] {
                if let Some(id) = id {
                    assert!(detail
                        .diff
                        .lines
                        .iter()
                        .any(|line| line.line_type == line_type
                            && line.content == format!("Subproject commit {id}")));
                }
            }
            assert!(detail.metadata.is_empty());
            assert!(detail.diff.image_preview.is_none());
        }
    }

    #[test]
    fn gitlink_is_reported_even_when_a_binary_type_change_has_no_patch() {
        let (_temp, repo) = create_test_repo();
        let id = repo.blob(b"\0binary").unwrap();
        let a = snapshot_entries(&repo, None, &[(b"file", id, FileMode::Blob)]);
        let b = snapshot_entries(&repo, Some(a), &[(b"file", id, FileMode::Commit)]);
        let files = compare_commit_files(&repo, a, b).unwrap();
        let detail = compare_commit_file(&repo, get(&files, "file")).unwrap();
        assert_eq!(detail.diff.lines[0].content, DIFF_UNAVAILABLE_MESSAGE);
        assert_eq!(
            detail.metadata,
            [
                "Mode 100644 → 160000".to_owned(),
                format!("+ Subproject commit {id}")
            ]
        );
    }

    #[cfg(feature = "image-preview")]
    #[test]
    fn selected_image_uses_snapshot_bytes_without_hiding_mode_metadata() {
        let (_temp, repo) = create_test_repo();
        let image_bytes = b"\x89PNG\r\n\x1a\n\0";
        let image = repo.blob(image_bytes).unwrap();
        let a = snapshot_entries(&repo, None, &[(b"image.png", image, FileMode::Blob)]);
        let b = snapshot_entries(
            &repo,
            Some(a),
            &[(b"image.png", image, FileMode::BlobExecutable)],
        );
        create_file(&repo, "image.png", "dirty worktree");
        let files = compare_commit_files(&repo, a, b).unwrap();
        let detail = compare_commit_file(&repo, get(&files, "image.png")).unwrap();
        assert_eq!(detail.metadata, ["Mode 100644 → 100755"]);
        assert_eq!(&*detail.diff.image_preview.unwrap().bytes, image_bytes);
    }

    #[test]
    fn exact_oids_must_identify_commits() {
        let (_temp, repo) = create_test_repo();
        let a = snapshot(&repo, None, &[("file.txt", "content\n")]);
        let tree = repo.find_commit(a).unwrap().tree_id();
        let annotated_tag = repo
            .tag_annotation_create(
                "annotated",
                &repo.find_commit(a).unwrap().into_object(),
                &repo.signature().unwrap(),
                "tag",
            )
            .unwrap();
        for invalid in [tree, annotated_tag, Oid::ZERO_SHA1] {
            assert!(compare_commit_files(&repo, invalid, a).is_err());
            assert!(compare_commit_files(&repo, a, invalid).is_err());
        }
    }
}
