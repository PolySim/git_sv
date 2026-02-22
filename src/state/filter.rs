//! État et logique de filtrage pour le graph de commits.

use crate::git::commit::CommitInfo;

/// Filtres applicables sur le graph de commits.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GraphFilter {
    /// Filtre par auteur (substring match, case-insensitive).
    pub author: Option<String>,
    /// Filtre par date de début (timestamp unix, inclus).
    pub date_from: Option<i64>,
    /// Filtre par date de fin (timestamp unix, inclus).
    pub date_to: Option<i64>,
    /// Filtre par chemin de fichier modifié.
    pub path: Option<String>,
    /// Filtre par texte dans le message de commit.
    pub message: Option<String>,
}

impl GraphFilter {
    /// Crée un nouveau filtre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Vérifie si au moins un critère de filtre est actif.
    pub fn is_active(&self) -> bool {
        self.author.is_some()
            || self.date_from.is_some()
            || self.date_to.is_some()
            || self.path.is_some()
            || self.message.is_some()
    }

    /// Réinitialise tous les filtres.
    pub fn clear(&mut self) {
        self.author = None;
        self.date_from = None;
        self.date_to = None;
        self.path = None;
        self.message = None;
    }

    /// Filtre une liste de commits selon les critères actifs.
    pub fn filter_commits(&self, commits: &[CommitInfo]) -> Vec<CommitInfo> {
        if !self.is_active() {
            return commits.to_vec();
        }

        commits
            .iter()
            .filter(|commit| self.matches(commit))
            .cloned()
            .collect()
    }

    /// Vérifie si un commit correspond aux critères de filtre.
    fn matches(&self, commit: &CommitInfo) -> bool {
        // Filtre par auteur
        if let Some(ref author_filter) = self.author {
            if !commit
                .author
                .to_lowercase()
                .contains(&author_filter.to_lowercase())
            {
                return false;
            }
        }

        // Filtre par date de début
        if let Some(date_from) = self.date_from {
            if commit.timestamp < date_from {
                return false;
            }
        }

        // Filtre par date de fin
        if let Some(date_to) = self.date_to {
            if commit.timestamp > date_to {
                return false;
            }
        }

        // Filtre par message
        if let Some(ref message_filter) = self.message {
            if !commit
                .message
                .to_lowercase()
                .contains(&message_filter.to_lowercase())
            {
                return false;
            }
        }

        // Note: Le filtre par chemin nécessite des informations supplémentaires
        // sur les fichiers modifiés par le commit, ce qui n'est pas dans CommitInfo.
        // Ce filtre sera appliqué lors de la récupération des commits.

        true
    }
}

/// État du popup de filtre.
#[derive(Debug, Clone, Default)]
pub struct FilterPopupState {
    /// Indique si le popup est ouvert.
    pub is_open: bool,
    /// Champ actuellement sélectionné.
    pub selected_field: FilterField,
    /// Valeur temporaire pour le champ auteur.
    pub author_input: String,
    /// Valeur temporaire pour le champ date de début (format: YYYY-MM-DD).
    pub date_from_input: String,
    /// Valeur temporaire pour le champ date de fin (format: YYYY-MM-DD).
    pub date_to_input: String,
    /// Valeur temporaire pour le champ chemin.
    pub path_input: String,
    /// Valeur temporaire pour le champ message.
    pub message_input: String,
}

impl FilterPopupState {
    /// Crée un nouvel état de popup fermé.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ouvre le popup avec les valeurs actuelles du filtre.
    pub fn open(&mut self, current_filter: &GraphFilter) {
        self.is_open = true;
        self.selected_field = FilterField::Author;
        self.author_input = current_filter.author.clone().unwrap_or_default();
        self.date_from_input = current_filter
            .date_from
            .map(timestamp_to_date_string)
            .unwrap_or_default();
        self.date_to_input = current_filter
            .date_to
            .map(timestamp_to_date_string)
            .unwrap_or_default();
        self.path_input = current_filter.path.clone().unwrap_or_default();
        self.message_input = current_filter.message.clone().unwrap_or_default();
    }

    /// Ferme le popup sans sauvegarder.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Passe au champ suivant.
    pub fn next_field(&mut self) {
        self.selected_field = match self.selected_field {
            FilterField::Author => FilterField::DateFrom,
            FilterField::DateFrom => FilterField::DateTo,
            FilterField::DateTo => FilterField::Path,
            FilterField::Path => FilterField::Message,
            FilterField::Message => FilterField::Author,
        };
    }

    /// Passe au champ précédent.
    pub fn previous_field(&mut self) {
        self.selected_field = match self.selected_field {
            FilterField::Author => FilterField::Message,
            FilterField::DateFrom => FilterField::Author,
            FilterField::DateTo => FilterField::DateFrom,
            FilterField::Path => FilterField::DateTo,
            FilterField::Message => FilterField::Path,
        };
    }

    /// Retourne la valeur du champ actuellement sélectionné.
    pub fn current_input(&self) -> &String {
        match self.selected_field {
            FilterField::Author => &self.author_input,
            FilterField::DateFrom => &self.date_from_input,
            FilterField::DateTo => &self.date_to_input,
            FilterField::Path => &self.path_input,
            FilterField::Message => &self.message_input,
        }
    }

    /// Retourne une référence mutable vers la valeur du champ actuellement sélectionné.
    pub fn current_input_mut(&mut self) -> &mut String {
        match self.selected_field {
            FilterField::Author => &mut self.author_input,
            FilterField::DateFrom => &mut self.date_from_input,
            FilterField::DateTo => &mut self.date_to_input,
            FilterField::Path => &mut self.path_input,
            FilterField::Message => &mut self.message_input,
        }
    }

    /// Applique les valeurs du popup à un GraphFilter.
    pub fn apply_to_filter(&self, filter: &mut GraphFilter) {
        filter.author = if self.author_input.is_empty() {
            None
        } else {
            Some(self.author_input.clone())
        };

        filter.date_from = parse_date(&self.date_from_input);
        filter.date_to = parse_date(&self.date_to_input).map(|t| t + 86399); // Fin de journée

        filter.path = if self.path_input.is_empty() {
            None
        } else {
            Some(self.path_input.clone())
        };

        filter.message = if self.message_input.is_empty() {
            None
        } else {
            Some(self.message_input.clone())
        };
    }
}

/// Champs du popup de filtre.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FilterField {
    #[default]
    Author,
    DateFrom,
    DateTo,
    Path,
    Message,
}

/// Convertit un timestamp unix en chaîne de date (YYYY-MM-DD).
fn timestamp_to_date_string(timestamp: i64) -> String {
    use chrono::{DateTime, Local};
    let datetime = DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.with_timezone(&Local))
        .unwrap_or_else(|| Local::now());
    datetime.format("%Y-%m-%d").to_string()
}

/// Parse une chaîne de date (YYYY-MM-DD) en timestamp unix.
fn parse_date(date_str: &str) -> Option<i64> {
    if date_str.is_empty() {
        return None;
    }

    use chrono::NaiveDate;
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .map(|date| date.and_hms_opt(0, 0, 0).unwrap().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;

    fn create_test_commit(author: &str, message: &str, timestamp: i64) -> CommitInfo {
        CommitInfo {
            oid: Oid::zero(),
            message: message.to_string(),
            author: author.to_string(),
            email: "test@example.com".to_string(),
            timestamp,
            parents: Vec::new(),
        }
    }

    #[test]
    fn test_filter_is_active() {
        let mut filter = GraphFilter::new();
        assert!(!filter.is_active());

        filter.author = Some("test".to_string());
        assert!(filter.is_active());

        filter.clear();
        assert!(!filter.is_active());
    }

    #[test]
    fn test_filter_by_author() {
        let commits = vec![
            create_test_commit("Alice", "First commit", 1000),
            create_test_commit("Bob", "Second commit", 2000),
            create_test_commit("Charlie", "Third commit", 3000),
        ];

        let mut filter = GraphFilter::new();
        filter.author = Some("ali".to_string());

        let filtered = filter.filter_commits(&commits);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].author, "Alice");
    }

    #[test]
    fn test_filter_by_message() {
        let commits = vec![
            create_test_commit("Alice", "Fix bug in login", 1000),
            create_test_commit("Bob", "Add feature X", 2000),
            create_test_commit("Charlie", "Fix another bug", 3000),
        ];

        let mut filter = GraphFilter::new();
        filter.message = Some("fix".to_string());

        let filtered = filter.filter_commits(&commits);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_date_range() {
        let commits = vec![
            create_test_commit("Alice", "Old commit", 1000),
            create_test_commit("Bob", "Middle commit", 5000),
            create_test_commit("Charlie", "Recent commit", 10000),
        ];

        let mut filter = GraphFilter::new();
        filter.date_from = Some(2000);
        filter.date_to = Some(8000);

        let filtered = filter.filter_commits(&commits);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].author, "Bob");
    }

    #[test]
    fn test_filter_combined() {
        let commits = vec![
            create_test_commit("Alice", "Fix bug", 1000),
            create_test_commit("Alice", "Add feature", 2000),
            create_test_commit("Bob", "Fix bug", 3000),
        ];

        let mut filter = GraphFilter::new();
        filter.author = Some("alice".to_string());
        filter.message = Some("fix".to_string());

        let filtered = filter.filter_commits(&commits);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].author, "Alice");
        assert_eq!(filtered[0].message, "Fix bug");
    }

    #[test]
    fn test_parse_date() {
        let ts = parse_date("2024-01-15");
        assert!(ts.is_some());

        // Date invalide
        assert!(parse_date("invalid").is_none());

        // Chaîne vide
        assert!(parse_date("").is_none());
    }

    #[test]
    fn test_popup_state_navigation() {
        let mut popup = FilterPopupState::new();

        assert_eq!(popup.selected_field, FilterField::Author);

        popup.next_field();
        assert_eq!(popup.selected_field, FilterField::DateFrom);

        popup.next_field();
        assert_eq!(popup.selected_field, FilterField::DateTo);

        popup.previous_field();
        assert_eq!(popup.selected_field, FilterField::DateFrom);
    }

    #[test]
    fn test_popup_apply_to_filter() {
        let mut popup = FilterPopupState::new();
        popup.author_input = "Alice".to_string();
        popup.message_input = "fix".to_string();
        popup.date_from_input = "2024-01-01".to_string();

        let mut filter = GraphFilter::new();
        popup.apply_to_filter(&mut filter);

        assert_eq!(filter.author, Some("Alice".to_string()));
        assert_eq!(filter.message, Some("fix".to_string()));
        assert!(filter.date_from.is_some());
        assert!(filter.date_to.is_none());
    }
}
