//! Actions de l'application organisées par domaine.

mod navigation;
mod git;
mod staging;
mod branch;
mod conflict;
mod search;
mod edit;

pub use navigation::NavigationAction;
pub use git::GitAction;
pub use staging::StagingAction;
pub use branch::BranchAction;
pub use conflict::ConflictAction;
pub use search::SearchAction;
pub use edit::EditAction;

use super::view::ViewMode;

/// Action principale de l'application.
/// 
/// Délègue vers des sous-enums spécialisés pour une meilleure organisation.
/// 
/// Note: Les variantes legacy ci-dessous sont maintenues pour compatibilité
/// et seront dépréciées dans une future version.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Quitter l'application
    Quit,
    
    /// Rafraîchir les données
    Refresh,
    
    /// Actions de navigation (nouvelle structure)
    Navigation(NavigationAction),
    
    /// Actions git (push, pull, fetch, etc.) (nouvelle structure)
    Git(GitAction),
    
    /// Actions de staging/commit (nouvelle structure)
    Staging(StagingAction),
    
    /// Actions sur les branches (nouvelle structure)
    Branch(BranchAction),
    
    /// Actions de résolution de conflits (nouvelle structure)
    Conflict(ConflictAction),
    
    /// Actions de recherche (nouvelle structure)
    Search(SearchAction),
    
    /// Actions d'édition de texte (nouvelle structure)
    Edit(EditAction),
    
    /// Changer de mode de vue
    SwitchView(ViewMode),
    
    /// Afficher/masquer l'aide
    ToggleHelp,
    
    /// Copier dans le presse-papier (nouvelle structure)
    CopyToClipboard,
    /// Copier le contenu du panneau (legacy - utiliser CopyToClipboard)
    CopyPanelContent,
    
    /// Sélectionner l'élément courant (Enter général)
    Select,
    
    /// Basculer le mode du panneau bas-gauche
    SwitchBottomMode,
    
    /// Fermer le panneau de branches
    CloseBranchPanel,
    
    /// Confirmer une action destructive
    ConfirmAction,
    
    /// Annuler une action destructive
    CancelAction,
    
    /// Navigation dans le merge picker
    MergePickerUp,
    MergePickerDown,
    MergePickerConfirm,
    MergePickerCancel,
    
    // ═══════════════════════════════════════════════════
    // Variantes legacy pour compatibilité ascendante
    // TODO: Migrer vers les sous-enums et supprimer ces variantes
    // ═══════════════════════════════════════════════════
    
    /// Navigation: Monter (legacy - utiliser Navigation(MoveUp))
    MoveUp,
    /// Navigation: Descendre (legacy - utiliser Navigation(MoveDown))
    MoveDown,
    /// Navigation: Page up (legacy - utiliser Navigation(PageUp))
    PageUp,
    /// Navigation: Page down (legacy - utiliser Navigation(PageDown))
    PageDown,
    /// Navigation: Début (legacy - utiliser Navigation(GoTop))
    GoTop,
    /// Navigation: Fin (legacy - utiliser Navigation(GoBottom))
    GoBottom,
    /// Navigation: Fichier up (legacy - utiliser Navigation(FileUp))
    FileUp,
    /// Navigation: Fichier down (legacy - utiliser Navigation(FileDown))
    FileDown,
    /// Navigation: Scroll diff up (legacy - utiliser Navigation(ScrollDiffUp))
    DiffScrollUp,
    /// Navigation: Scroll diff down (legacy - utiliser Navigation(ScrollDiffDown))
    DiffScrollDown,
    
    /// Git: Push (legacy - utiliser Git(Push))
    GitPush,
    /// Git: Pull (legacy - utiliser Git(Pull))
    GitPull,
    /// Git: Fetch (legacy - utiliser Git(Fetch))
    GitFetch,
    /// Git: Cherry-pick (legacy - utiliser Git(CherryPick))
    CherryPick,
    /// Git: Amend (legacy - utiliser Git(AmendCommit))
    AmendCommit,
    /// Git: Ouvrir blame (legacy - utiliser Git(OpenBlame))
    OpenBlame,
    /// Git: Fermer blame (legacy - utiliser Git(CloseBlame))
    CloseBlame,
    /// Git: Aller au commit blame (legacy - utiliser Git(JumpToBlameCommit))
    JumpToBlameCommit,
    /// Git: Commit prompt (legacy - utiliser Git(CommitPrompt))
    CommitPrompt,
    /// Git: Stash prompt (legacy - utiliser Git(StashPrompt))
    StashPrompt,
    /// Git: Merge prompt (legacy - utiliser Git(MergePrompt))
    MergePrompt,
    /// Git: Branch list (legacy - utiliser Git(BranchList))
    BranchList,
    
    /// Staging: Stage file (legacy - utiliser Staging(StageFile))
    StageFile,
    /// Staging: Unstage file (legacy - utiliser Staging(UnstageFile))
    UnstageFile,
    /// Staging: Stage all (legacy - utiliser Staging(StageAll))
    StageAll,
    /// Staging: Unstage all (legacy - utiliser Staging(UnstageAll))
    UnstageAll,
    /// Staging: Switch focus (legacy - utiliser Staging(SwitchFocus))
    SwitchStagingFocus,
    /// Staging: Start commit message (legacy - utiliser Staging(StartCommitMessage))
    StartCommitMessage,
    /// Staging: Confirm commit (legacy - utiliser Staging(ConfirmCommit))
    ConfirmCommit,
    /// Staging: Cancel commit message (legacy - utiliser Staging(CancelCommit))
    CancelCommitMessage,
    /// Staging: Discard file (legacy - utiliser Staging(DiscardFile))
    DiscardFile,
    /// Staging: Discard all (legacy - utiliser Staging(DiscardAll))
    DiscardAll,
    /// Staging: Stash selected file (legacy - utiliser Staging(StashSelectedFile))
    StashSelectedFile,
    /// Staging: Stash unstaged files (legacy - utiliser Staging(StashUnstagedFiles))
    StashUnstagedFiles,
    
    /// Branch: Checkout (legacy - utiliser Branch(Checkout))
    BranchCheckout,
    /// Branch: Create (legacy - utiliser Branch(Create))
    BranchCreate,
    /// Branch: Delete (legacy - utiliser Branch(Delete))
    BranchDelete,
    /// Branch: Rename (legacy - utiliser Branch(Rename))
    BranchRename,
    /// Branch: Toggle remote (legacy - utiliser Branch(ToggleRemote))
    ToggleRemoteBranches,
    /// Branch: Worktree create (legacy - utiliser Branch(WorktreeCreate))
    WorktreeCreate,
    /// Branch: Worktree remove (legacy - utiliser Branch(WorktreeRemove))
    WorktreeRemove,
    /// Branch: Stash apply (legacy - utiliser Branch(StashApply))
    StashApply,
    /// Branch: Stash pop (legacy - utiliser Branch(StashPop))
    StashPop,
    /// Branch: Stash drop (legacy - utiliser Branch(StashDrop))
    StashDrop,
    /// Branch: Stash save (legacy - utiliser Branch(StashSave))
    StashSave,
    /// Branch: Next section (legacy - utiliser Branch(NextSection))
    NextSection,
    /// Branch: Prev section (legacy - utiliser Branch(PrevSection))
    PrevSection,
    /// Branch: Confirm input (legacy - utiliser Branch(ConfirmInput))
    ConfirmInput,
    /// Branch: Cancel input (legacy - utiliser Branch(CancelInput))
    CancelInput,
    
    /// Search: Ouvrir (legacy - utiliser Search(Open))
    OpenSearch,
    /// Search: Fermer (legacy - utiliser Search(Close))
    CloseSearch,
    /// Search: Changer type (legacy - utiliser Search(ChangeType))
    ChangeSearchType,
    /// Search: Résultat suivant (legacy - utiliser Search(NextResult))
    NextSearchResult,
    /// Search: Résultat précédent (legacy - utiliser Search(PreviousResult))
    PrevSearchResult,
    
    /// Edit: Insérer caractère (legacy - utiliser Edit(InsertChar(c)))
    InsertChar(char),
    /// Edit: Supprimer caractère (legacy - utiliser Edit(DeleteCharBefore))
    DeleteChar,
    /// Edit: Curseur gauche (legacy - utiliser Edit(CursorLeft))
    MoveCursorLeft,
    /// Edit: Curseur droite (legacy - utiliser Edit(CursorRight))
    MoveCursorRight,
    
    /// Vue: Graph (legacy - utiliser SwitchView(Graph))
    SwitchToGraph,
    /// Vue: Staging (legacy - utiliser SwitchView(Staging))
    SwitchToStaging,
    /// Vue: Branches (legacy - utiliser SwitchView(Branches))
    SwitchToBranches,
    /// Vue: Conflits (legacy - utiliser SwitchView(Conflicts))
    SwitchToConflicts,
    
    /// Conflit: Fichier précédent (legacy - utiliser Conflict(PreviousFile))
    ConflictPrevFile,
    /// Conflit: Fichier suivant (legacy - utiliser Conflict(NextFile))
    ConflictNextFile,
    /// Conflit: Section précédente (legacy - utiliser Conflict(PreviousSection))
    ConflictPrevSection,
    /// Conflit: Section suivante (legacy - utiliser Conflict(NextSection))
    ConflictNextSection,
    /// Conflit: Changer panneau (legacy - utiliser Conflict(SwitchPanel))
    ConflictSwitchPanelForward,
    ConflictSwitchPanelReverse,
    /// Conflit: Accepter ours fichier (legacy - utiliser Conflict(AcceptOursFile))
    ConflictFileChooseOurs,
    /// Conflit: Accepter theirs fichier (legacy - utiliser Conflict(AcceptTheirsFile))
    ConflictFileChooseTheirs,
    /// Conflit: Accepter both (legacy - utiliser Conflict(AcceptBoth))
    ConflictChooseBoth,
    /// Conflit: Valider merge (legacy - utiliser Conflict(FinalizeMerge))
    ConflictFinalize,
    ConflictValidateMerge,
    /// Conflit: Abandonner merge (legacy - utiliser Conflict(AbortMerge))
    ConflictAbort,
    /// Conflit: Quitter vue (legacy - utiliser Conflict(LeaveView))
    ConflictLeaveView,
    /// Conflit: Enter resolve (legacy - utiliser Conflict(EnterResolve))
    ConflictEnterResolve,
    /// Conflit: Set mode file (legacy - utiliser Conflict(SetModeFile))
    ConflictSetModeFile,
    /// Conflit: Set mode block (legacy - utiliser Conflict(SetModeBlock))
    ConflictSetModeBlock,
    /// Conflit: Set mode line (legacy - utiliser Conflict(SetModeLine))
    ConflictSetModeLine,
    /// Conflit: Toggle line (legacy - utiliser Conflict(ToggleLine))
    ConflictToggleLine,
    /// Conflit: Line up (legacy - utiliser Conflict(LineUp))
    ConflictLineUp,
    /// Conflit: Line down (legacy - utiliser Conflict(LineDown))
    ConflictLineDown,
    /// Conflit: Scroll résultat up (legacy - utiliser Conflict(ResultScrollUp))
    ConflictResultScrollUp,
    /// Conflit: Scroll résultat down (legacy - utiliser Conflict(ResultScrollDown))
    ConflictResultScrollDown,
    /// Conflit: Start editing (legacy - utiliser Conflict(StartEdit))
    ConflictStartEditing,
    /// Conflit: Stop editing (legacy - utiliser Conflict(CancelEdit))
    ConflictStopEditing,
    /// Conflit: Edit insert char (legacy - utiliser Conflict(EditInsertChar(c)))
    ConflictEditInsertChar(char),
    /// Conflit: Edit backspace (legacy - utiliser Conflict(EditBackspace))
    ConflictEditBackspace,
    /// Conflit: Edit delete (legacy - utiliser Conflict(EditDelete))
    ConflictEditDelete,
    /// Conflit: Edit cursor up (legacy - utiliser Conflict(EditCursorUp))
    ConflictEditCursorUp,
    ConflictEditCursorDown,
    ConflictEditCursorLeft,
    ConflictEditCursorRight,
    /// Conflit: Edit newline (legacy - utiliser Conflict(EditNewline))
    ConflictEditNewline,
    /// Conflit: Mark resolved (legacy - utiliser Conflict(MarkResolved))
    ConflictResolveFile,
    
    /// Aucune action (événement ignoré)
    None,
}
