use super::graph::{CommitNode, GraphRow};

/// Type de recherche à effectuer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SearchType {
    /// Recherche par message de commit.
    #[default]
    Message,
    /// Recherche par auteur.
    Author,
    /// Recherche par hash (commence par...).
    Hash,
}

/// Filtre les commits du graphe selon une requête et un type de recherche.
pub fn filter_commits(graph: &[GraphRow], query: &str, search_type: SearchType) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();

    graph
        .iter()
        .enumerate()
        .filter(|(_, row)| match_commit(&row.node, &query_lower, &search_type))
        .map(|(idx, _)| idx)
        .collect()
}

/// Vérifie si un commit correspond à la requête selon le type de recherche.
fn match_commit(commit: &CommitNode, query_lower: &str, search_type: &SearchType) -> bool {
    match search_type {
        SearchType::Message => commit.message.to_lowercase().contains(query_lower),
        SearchType::Author => commit.author.to_lowercase().contains(query_lower),
        SearchType::Hash => {
            // Recherche par préfixe du hash (court ou complet)
            let hash_str = commit.oid.to_string();
            hash_str.starts_with(query_lower) || commit.short_hash().starts_with(query_lower)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;

    fn create_test_node(message: &str, author: &str, hash: &str) -> CommitNode {
        CommitNode {
            oid: Oid::from_str(hash).unwrap(),
            message: message.to_string(),
            author: author.to_string(),
            timestamp: 0,
            parents: Vec::new(),
            refs: Vec::new(),
            branch_name: None,
            column: 0,
            color_index: 0,
        }
    }

    fn create_test_row(message: &str, author: &str, hash: &str) -> GraphRow {
        GraphRow {
            node: create_test_node(message, author, hash),
            cells: Vec::new(),
            connection: None,
        }
    }

    #[test]
    fn test_filter_by_message() {
        let graph = vec![
            create_test_row(
                "Add feature X",
                "Alice",
                "1234567890123456789012345678901234567890",
            ),
            create_test_row(
                "Fix bug Y",
                "Bob",
                "abcdef1234567890123456789012345678901234",
            ),
            create_test_row(
                "Update docs",
                "Alice",
                "fedcba9876543210987654321098765432109876",
            ),
        ];

        let results = filter_commits(&graph, "feature", SearchType::Message);
        assert_eq!(results, vec![0]);

        let results = filter_commits(&graph, "Fix", SearchType::Message);
        assert_eq!(results, vec![1]);

        let results = filter_commits(&graph, "alice", SearchType::Message);
        assert_eq!(results, Vec::<usize>::new());
    }

    #[test]
    fn test_filter_by_author() {
        let graph = vec![
            create_test_row(
                "Add feature X",
                "Alice",
                "1234567890123456789012345678901234567890",
            ),
            create_test_row(
                "Fix bug Y",
                "Bob",
                "abcdef1234567890123456789012345678901234",
            ),
            create_test_row(
                "Update docs",
                "Alice Smith",
                "fedcba9876543210987654321098765432109876",
            ),
        ];

        let results = filter_commits(&graph, "alice", SearchType::Author);
        assert_eq!(results, vec![0, 2]);

        let results = filter_commits(&graph, "Bob", SearchType::Author);
        assert_eq!(results, vec![1]);
    }

    #[test]
    fn test_filter_by_hash() {
        let graph = vec![
            create_test_row(
                "Add feature X",
                "Alice",
                "1234567890123456789012345678901234567890",
            ),
            create_test_row(
                "Fix bug Y",
                "Bob",
                "abcdef1234567890123456789012345678901234",
            ),
            create_test_row(
                "Update docs",
                "Alice",
                "fedcba9876543210987654321098765432109876",
            ),
        ];

        let results = filter_commits(&graph, "1234567", SearchType::Hash);
        assert_eq!(results, vec![0]);

        let results = filter_commits(&graph, "abcdef", SearchType::Hash);
        assert_eq!(results, vec![1]);

        let results = filter_commits(&graph, "xyz", SearchType::Hash);
        assert_eq!(results, Vec::<usize>::new());
    }

    #[test]
    fn test_empty_query() {
        let graph = vec![create_test_row(
            "Add feature X",
            "Alice",
            "1234567890123456789012345678901234567890",
        )];

        let results = filter_commits(&graph, "", SearchType::Message);
        assert_eq!(results, Vec::<usize>::new());
    }
}
