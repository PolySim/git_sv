use crate::git::conflict::MergeResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSuccess {
    pub branch_name: String,
    pub remote_name: String,
    pub force: bool,
    pub upstream_set: bool,
}

impl PushSuccess {
    pub fn flash_message(&self) -> String {
        match (self.force, self.upstream_set) {
            (true, true) => format!(
                "Force push de '{}' vers {}/{} (upstream configuré) ✓",
                self.branch_name, self.remote_name, self.branch_name
            ),
            (true, false) => {
                format!(
                    "Force push de '{}' vers {} ✓",
                    self.branch_name, self.remote_name
                )
            }
            (false, true) => format!(
                "Push de '{}' vers {}/{} (upstream configuré) ✓",
                self.branch_name, self.remote_name, self.branch_name
            ),
            (false, false) => {
                format!("Push de '{}' vers {} ✓", self.branch_name, self.remote_name)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchSuccess {
    pub remote_name: String,
}

impl FetchSuccess {
    pub fn flash_message(&self) -> String {
        format!("Fetch depuis '{}' réussi ✓", self.remote_name)
    }
}

pub fn flash_message_for_pull_result(result: &MergeResult) -> Option<String> {
    match result {
        MergeResult::UpToDate => Some("Déjà à jour ✓".to_string()),
        MergeResult::FastForward => Some("Pull (fast-forward) réussi ✓".to_string()),
        MergeResult::Success => Some("Pull réussi ✓".to_string()),
        MergeResult::Conflicts(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_success_flash_message_variants() {
        let normal = PushSuccess {
            branch_name: "main".to_string(),
            remote_name: "origin".to_string(),
            force: false,
            upstream_set: false,
        };
        assert!(normal
            .flash_message()
            .contains("Push de 'main' vers origin"));

        let force = PushSuccess {
            branch_name: "feature".to_string(),
            remote_name: "origin".to_string(),
            force: true,
            upstream_set: true,
        };
        assert!(force.flash_message().contains("Force push"));
        assert!(force.flash_message().contains("upstream configuré"));
    }

    #[test]
    fn test_fetch_success_flash_message() {
        let success = FetchSuccess {
            remote_name: "origin".to_string(),
        };

        assert_eq!(success.flash_message(), "Fetch depuis 'origin' réussi ✓");
    }

    #[test]
    fn test_pull_result_flash_message() {
        assert_eq!(
            flash_message_for_pull_result(&MergeResult::UpToDate),
            Some("Déjà à jour ✓".to_string())
        );
        assert_eq!(
            flash_message_for_pull_result(&MergeResult::Conflicts(Vec::new())),
            None
        );
    }
}
