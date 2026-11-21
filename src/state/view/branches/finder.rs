//! Sélection temporaire dans les branches visibles, sans modifier la liste principale.

use super::{BranchesFocus, BranchesSection, BranchesViewState, SelectedBranch};
use crate::state::action::BranchFinderAction;

const MAX_QUERY_CHARS: usize = 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct BranchFinder {
    pub(crate) query: String,
    pub(crate) matches: Vec<SelectedBranch>,
    pub(crate) selected: Option<usize>,
}

impl BranchFinder {
    fn candidate(&self) -> Option<SelectedBranch> {
        self.matches.get(self.selected?).copied()
    }

    fn navigate(&mut self, action: BranchFinderAction) {
        let Some(last) = self.matches.len().checked_sub(1) else {
            return;
        };
        self.selected = Some(match action {
            BranchFinderAction::Up => self.selected.map_or(last, |index| index.saturating_sub(1)),
            BranchFinderAction::Down => self.selected.map_or(0, |index| (index + 1).min(last)),
            BranchFinderAction::First => 0,
            BranchFinderAction::Last => last,
            _ => return,
        });
    }
}

impl BranchesViewState {
    pub(crate) fn finder(&self) -> Option<&BranchFinder> {
        self.finder.as_ref()
    }

    pub(crate) fn apply_finder(&mut self, action: BranchFinderAction) {
        use BranchFinderAction::*;
        if self.section != BranchesSection::Branches || self.focus == BranchesFocus::Input {
            return;
        }
        if action == Open && self.finder.is_none() {
            let candidate = self
                .selected_branch_info()
                .map(|(branch, selected)| (branch.name.clone(), selected.is_remote()));
            self.finder = Some(BranchFinder::default());
            self.refresh_finder(candidate);
            self.select_first_match_if_needed();
            return;
        }
        let Some(finder) = &mut self.finder else {
            return;
        };
        match action {
            Open => {}
            Cancel => self.finder = None,
            Accept => {
                if let Some(target) = finder.candidate().filter(|&s| self.branch_at(s).is_some()) {
                    self.selected_branch = Some(target);
                    self.focus = BranchesFocus::List;
                    self.finder = None;
                }
            }
            Up | Down | First | Last => finder.navigate(action),
            Insert(_) | Backspace | ClearQuery => self.edit_finder_query(action),
        }
    }

    fn edit_finder_query(&mut self, action: BranchFinderAction) {
        let Some(finder) = &mut self.finder else {
            return;
        };
        match action {
            BranchFinderAction::Insert(value) => {
                let remaining = MAX_QUERY_CHARS.saturating_sub(finder.query.chars().count());
                finder.query.extend(
                    value
                        .chars()
                        .filter_map(|c| {
                            if c.is_whitespace() {
                                Some(' ')
                            } else {
                                (!c.is_control()).then_some(c)
                            }
                        })
                        .take(remaining),
                );
            }
            BranchFinderAction::Backspace => {
                finder.query.pop();
            }
            BranchFinderAction::ClearQuery => finder.query.clear(),
            _ => return,
        }
        self.refresh_finder(None);
        self.select_first_match_if_needed();
    }

    fn select_first_match_if_needed(&mut self) {
        if let Some(finder) = &mut self.finder {
            if finder.selected.is_none() {
                finder.navigate(BranchFinderAction::First);
            }
        }
    }

    pub(super) fn finder_candidate_key(&self) -> Option<(String, bool)> {
        let candidate = self.finder.as_ref()?.candidate()?;
        let branch = self.branch_at(candidate)?;
        Some((branch.name.clone(), candidate.is_remote()))
    }

    pub(super) fn refresh_finder(&mut self, candidate: Option<(String, bool)>) {
        let Some(finder) = &mut self.finder else {
            return;
        };
        let query = finder.query.trim().to_lowercase();
        finder.matches.clear();
        // Si le candidat a disparu au rafraîchissement, ne pas en valider un autre par surprise.
        finder.selected = None;
        let groups = [
            (
                false,
                &self.local_branches,
                SelectedBranch::Local as fn(usize) -> SelectedBranch,
            ),
            (true, &self.remote_branches, SelectedBranch::Remote),
        ];
        for (remote, branches, index_to_branch) in groups {
            if remote && !self.show_remote {
                continue;
            }
            for (index, branch) in branches.iter().enumerate() {
                if query.is_empty() || branch.name.to_lowercase().contains(&query) {
                    if candidate.as_ref().is_some_and(|(name, is_remote)| {
                        *is_remote == remote && *name == branch.name
                    }) {
                        finder.selected = Some(finder.matches.len());
                    }
                    finder.matches.push(index_to_branch(index));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branch::BranchInfo;
    use BranchFinderAction::*;

    fn branch(name: &str, remote: bool) -> BranchInfo {
        BranchInfo::simple(name.into(), false, remote)
    }

    #[test]
    fn finder_matches_visible_branches_and_cancel_leaves_selection_and_focus_alone() {
        let mut state = BranchesViewState::new();
        state.replace_branches(
            vec![branch("main", false), branch("release/Été", false)],
            vec![branch("origin/release/été", true)],
        );
        state.focus = BranchesFocus::Detail;
        state.apply_finder(Open);
        state.apply_finder(Insert("\tÉTÉ\r\n".into()));
        assert_eq!(state.finder().unwrap().matches, [SelectedBranch::Local(1)]);
        state.toggle_remote();
        assert_eq!(state.finder().unwrap().matches.len(), 2);
        state.apply_finder(Down);
        assert_eq!(
            state.finder().unwrap().candidate(),
            Some(SelectedBranch::Remote(0))
        );
        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
        state.apply_finder(Cancel);
        assert!(state.finder().is_none());
        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
        assert_eq!(state.focus, BranchesFocus::Detail);
    }

    #[test]
    fn finder_preserves_candidate_name_and_kind_when_indices_change() {
        let mut state = BranchesViewState::new();
        state.toggle_remote();
        state.replace_branches(
            vec![branch("origin/topic", false)],
            vec![branch("origin/topic", true)],
        );
        state.apply_finder(Open);
        state.apply_finder(Insert("topic".into()));
        state.apply_finder(Down);
        state.replace_branches(
            vec![branch("origin/topic", false)],
            vec![
                branch("origin/other-topic", true),
                branch("origin/topic", true),
            ],
        );
        assert_eq!(
            state.finder().unwrap().candidate(),
            Some(SelectedBranch::Remote(1))
        );
        state.apply_finder(Accept);
        assert_eq!(state.selected_branch, Some(SelectedBranch::Remote(1)));
        assert_eq!(state.focus, BranchesFocus::List);
        assert!(state.finder().is_none());
    }

    #[test]
    fn finder_does_not_accept_an_empty_result_or_retarget_a_removed_candidate() {
        let mut state = BranchesViewState::new();
        state.replace_branches(vec![branch("main", false), branch("topic", false)], vec![]);
        state.apply_finder(Open);
        state.apply_finder(Insert("no match".into()));
        state.apply_finder(Accept);
        assert!(state.finder().unwrap().matches.is_empty());
        state.apply_finder(ClearQuery);
        state.apply_finder(Down);
        state.replace_branches(vec![branch("main", false), branch("other", false)], vec![]);
        state.apply_finder(Accept);
        assert!(state.finder().unwrap().candidate().is_none());
        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
        state.apply_finder(Last);
        state.apply_finder(Accept);
        assert_eq!(state.selected_branch().unwrap().name, "other");
    }
}
