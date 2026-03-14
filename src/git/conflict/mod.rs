//! Resolution de conflits de merge : detection, modes (fichier/bloc/ligne), finalisation.

mod content;
mod merge_files;
mod parse;
mod repo_state;
mod resolve;
mod types;

pub use content::{
    all_sections_resolved, apply_resolved_content, generate_resolved_content_with_source,
};
pub use parse::{list_conflict_files, parse_conflict_file};
pub use repo_state::{abort_merge, finalize_merge, get_current_branch_name, is_merging};
pub use resolve::{resolve_file_with_strategy, resolve_special_file};
pub use types::{
    ConflictFile, ConflictResolution, ConflictResolutionMode, ConflictSection, ConflictType,
    LineLevelResolution, LineSource, MergeFile, MergeResult, ResolvedLine,
};
