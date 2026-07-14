//! Construction du graphe de commits (colonnes, connexions, couleurs).

mod layout;
mod refs;
mod types;

use std::collections::HashMap;

use git2::Repository;

use super::commit::CommitInfo;
use crate::error::Result;

use self::layout::{
    assign_parent_columns, build_commit_cells, build_connection_row, determine_branch_name,
    determine_color_index, find_or_assign_column,
};
use self::refs::collect_refs;

use self::types::ColumnState;
pub use self::types::{
    CommitNode, ConnectionRow, EdgeType, GraphCell, GraphRow, RefInfo, RefType, GRAPH_COLOR_COUNT,
};

/// Construit le graphe de commits avec placement en colonnes et edges de connexion.
pub fn build_graph(repo: &Repository, commits: &[CommitInfo]) -> Result<Vec<GraphRow>> {
    let mut rows = Vec::with_capacity(commits.len());
    let mut active_columns: Vec<ColumnState> = Vec::new();
    let mut branch_colors: HashMap<String, usize> = HashMap::new();
    let mut next_color_index: usize = 0;
    let refs_map = collect_refs(repo)?;

    for (commit_idx, ci) in commits.iter().enumerate() {
        let oid = ci.oid;
        let (column, merged_columns) = find_or_assign_column(&mut active_columns, oid);
        let refs = refs_map.get(&oid).cloned().unwrap_or_default();

        let color_index = determine_color_index(
            column,
            &refs,
            &mut branch_colors,
            &mut next_color_index,
            &active_columns,
        );

        let branch_name = determine_branch_name(column, &refs, &active_columns);

        if column < active_columns.len() {
            active_columns[column].color_index = color_index;
            if active_columns[column].branch_name.is_none() && branch_name.is_some() {
                active_columns[column].branch_name = branch_name.clone();
            }
        }

        let node = CommitNode {
            oid,
            message: ci.message.clone(),
            author: ci.author.clone(),
            timestamp: ci.timestamp,
            parents: ci.parents.clone(),
            refs,
            branch_name: branch_name.clone(),
            column,
            color_index,
        };

        let cells = build_commit_cells(column, &active_columns, &merged_columns);

        if column < active_columns.len() {
            active_columns[column].expected_oid = None;
        }

        let parent_assignments =
            assign_parent_columns(&mut active_columns, column, ci, color_index);

        while active_columns
            .last()
            .is_some_and(|state| state.expected_oid.is_none())
        {
            active_columns.pop();
        }

        let connection = if commit_idx + 1 < commits.len() {
            Some(build_connection_row(&active_columns, &parent_assignments))
        } else {
            None
        };

        rows.push(GraphRow {
            node,
            cells,
            connection,
        });
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;

    use crate::git::graph::layout::{
        assign_new_column, determine_color_index, find_or_assign_column,
    };
    use crate::git::tests::test_utils::*;

    #[test]
    fn test_build_graph_linear() {
        let (_temp_dir, repo) = create_test_repo();

        let oid_a = commit_file(&repo, "file.txt", "A", "First commit");
        let oid_b = commit_file(&repo, "file.txt", "B", "Second commit");
        let oid_c = commit_file(&repo, "file.txt", "C", "Third commit");

        let commits = vec![
            CommitInfo::from_git2_commit(&repo.find_commit(oid_c).unwrap()),
            CommitInfo::from_git2_commit(&repo.find_commit(oid_b).unwrap()),
            CommitInfo::from_git2_commit(&repo.find_commit(oid_a).unwrap()),
        ];

        let graph = build_graph(&repo, &commits).unwrap();

        assert_eq!(graph.len(), 3);
        assert_eq!(graph[0].node.column, 0);
        assert_eq!(graph[1].node.column, 0);
        assert_eq!(graph[2].node.column, 0);
    }

    #[test]
    fn test_find_or_assign_column() {
        let mut columns: Vec<ColumnState> = vec![];
        let oid1 = Oid::from_bytes(&[1; 20]).unwrap();
        let oid2 = Oid::from_bytes(&[2; 20]).unwrap();

        let (col1, merges1) = find_or_assign_column(&mut columns, oid1);
        assert_eq!(col1, 0);
        assert!(merges1.is_empty());
        assert_eq!(columns.len(), 1);

        let (col2, merges2) = find_or_assign_column(&mut columns, oid2);
        assert_eq!(col2, 1);
        assert!(merges2.is_empty());
        assert_eq!(columns.len(), 2);

        let (col1_again, merges3) = find_or_assign_column(&mut columns, oid1);
        assert_eq!(col1_again, 0);
        assert!(merges3.is_empty());
    }

    #[test]
    fn test_find_or_assign_column_clears_duplicate_expected_oid() {
        let oid = Oid::from_bytes(&[42; 20]).unwrap();
        let mut columns = vec![
            ColumnState {
                expected_oid: Some(oid),
                color_index: 0,
                branch_name: Some("main".to_string()),
            },
            ColumnState {
                expected_oid: Some(oid),
                color_index: 1,
                branch_name: Some("feature".to_string()),
            },
        ];

        let (col, merged_columns) = find_or_assign_column(&mut columns, oid);

        assert_eq!(col, 0);
        assert_eq!(merged_columns, vec![1]);
        assert_eq!(columns[0].expected_oid, Some(oid));
        assert_eq!(columns[1].expected_oid, None);
        assert_eq!(columns[1].branch_name, None);
    }

    #[test]
    fn test_assign_new_column_reuse() {
        let mut columns: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None,
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
        ];

        let new_oid = Oid::from_bytes(&[3; 20]).unwrap();
        let col = assign_new_column(&mut columns, new_oid);

        assert_eq!(col, 1);
        assert_eq!(columns[1].expected_oid, Some(new_oid));
    }

    #[test]
    fn test_determine_color_index() {
        let mut branch_colors: HashMap<String, usize> = HashMap::new();
        let mut next_color: usize = 0;
        let columns: Vec<ColumnState> = vec![];

        let color = determine_color_index(0, &[], &mut branch_colors, &mut next_color, &columns);
        assert_eq!(color, 0);

        let color2 = determine_color_index(
            0,
            &vec![RefInfo::new("feature", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color2, 0);
        assert_eq!(next_color, 1);

        let color3 = determine_color_index(
            0,
            &vec![RefInfo::new("feature", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color3, 0);

        let color4 = determine_color_index(
            0,
            &vec![RefInfo::new("main", RefType::LocalBranch)],
            &mut branch_colors,
            &mut next_color,
            &columns,
        );
        assert_eq!(color4, 1);
        assert_eq!(next_color, 2);
    }

    #[test]
    fn test_collect_refs() {
        let (_temp_dir, repo) = create_test_repo();
        let oid = commit_file(&repo, "test.txt", "content", "Initial commit");

        let commit = repo.find_commit(oid).unwrap();
        repo.branch("feature", &commit, false).unwrap();

        let refs_map = collect_refs(&repo).unwrap();

        assert!(!refs_map.is_empty());

        if let Some(refs) = refs_map.get(&oid) {
            assert!(refs.iter().any(|r| r.name.contains("main")));
            assert!(refs.iter().any(|r| r.name.contains("feature")));
        } else {
            panic!("Commit non trouvé dans refs_map");
        }
    }

    #[test]
    fn test_column_compaction() {
        let mut active_columns: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 1,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None,
                color_index: 0,
                branch_name: None,
            },
        ];

        while active_columns
            .last()
            .is_some_and(|s| s.expected_oid.is_none())
        {
            active_columns.pop();
        }

        assert_eq!(active_columns.len(), 2);
        assert!(active_columns[0].expected_oid.is_some());
        assert!(active_columns[1].expected_oid.is_some());

        let mut active_columns2: Vec<ColumnState> = vec![
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[1; 20]).unwrap()),
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: None,
                color_index: 0,
                branch_name: None,
            },
            ColumnState {
                expected_oid: Some(Oid::from_bytes(&[2; 20]).unwrap()),
                color_index: 1,
                branch_name: None,
            },
        ];

        while active_columns2
            .last()
            .is_some_and(|s| s.expected_oid.is_none())
        {
            active_columns2.pop();
        }

        assert_eq!(active_columns2.len(), 3);
    }

    #[test]
    fn test_ref_classification() {
        let (_temp_dir, repo) = create_test_repo();
        let oid = commit_file(&repo, "test.txt", "content", "Initial commit");

        let commit = repo.find_commit(oid).unwrap();
        repo.branch("feature", &commit, false).unwrap();
        repo.tag(
            "v1.0",
            &commit.clone().into_object(),
            &git2::Signature::now("Test", "test@test.com").unwrap(),
            "Version 1.0",
            false,
        )
        .unwrap();

        let refs_map = collect_refs(&repo).unwrap();
        let commit_refs = refs_map
            .get(&oid)
            .expect("Le commit devrait avoir des refs");

        assert!(commit_refs
            .iter()
            .any(|r| r.ref_type == RefType::Head && r.name == "main"));
        assert!(commit_refs
            .iter()
            .any(|r| r.ref_type == RefType::LocalBranch && r.name == "main"));
        assert!(commit_refs
            .iter()
            .any(|r| r.ref_type == RefType::LocalBranch && r.name == "feature"));
        assert!(commit_refs
            .iter()
            .any(|r| r.ref_type == RefType::Tag && r.name == "v1.0"));
    }
}
