//! Resolution de conflits de merge : detection, modes (fichier/bloc/ligne), finalisation.

#![allow(dead_code)]
#![allow(unused_imports)]

mod content;
mod merge_files;
mod parse;
mod repo_state;
mod resolve;
mod types;

pub use content::{
    all_sections_resolved, apply_resolved_content, generate_resolved_content,
    generate_resolved_content_with_source,
};
pub use merge_files::{
    count_unresolved_files, count_unresolved_merge_files, count_unresolved_sections,
    list_all_merge_files, update_file_resolved_status,
};
pub use parse::{list_conflict_files, parse_conflict_file};
pub use repo_state::{
    abort_merge, finalize_merge, get_current_branch_name, get_merge_branch_name, has_conflicts,
    is_merging,
};
pub use resolve::{resolve_file, resolve_file_with_strategy, resolve_special_file};
pub use types::{
    ConflictFile, ConflictResolution, ConflictResolutionMode, ConflictSection, ConflictType,
    LineLevelResolution, LineResolution, LineSource, MergeFile, MergeResult, ResolutionSide,
    ResolvedLine,
};
