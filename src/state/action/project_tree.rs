//! Actions propres à la vue arborescence.

use super::EditAction;

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectTreeAction {
    ToggleSelected,
    ExpandSelected,
    CollapseSelected,
    ActivateTreeEntry(usize),
    FocusTree,
    FocusHistory,
    FocusChangedFiles,
    FocusDiff,
    SelectTreeEntry(usize),
    SelectSearchResult(usize),
    SelectHistoryEntry(usize),
    SelectChangedFile(usize),
    OpenSearch,
    CloseSearch,
    ConfirmSearch,
    SearchNext,
    SearchPrevious,
    EditSearch(EditAction),
}
