//! Marque A et session de comparaison indépendante des sélections du graphe.

use git2::Oid;

use crate::error::Result;
use crate::git::diff::{ComparisonDelta, ComparisonDetail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonAction {
    Mark,
    Clear,
    Open,
    Close,
    FileUp,
    FileDown,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Left,
    Right,
}

/// Les OIDs et la liste restent fixes, même si les références Git changent.
pub struct ComparisonSession {
    pub a: Oid,
    pub b: Oid,
    pub files: Result<Vec<ComparisonDelta>>,
    pub selected: usize,
    pub file_offset: usize,
    pub diff: Option<Result<ComparisonDetail>>,
    pub scroll: usize,
    pub horizontal: usize,
    pub page_height: usize,
    pub max_scroll: usize,
    pub max_horizontal: usize,
    pub content_width: usize,
}

impl ComparisonSession {
    pub fn new(a: Oid, b: Oid, files: Result<Vec<ComparisonDelta>>) -> Self {
        Self {
            a,
            b,
            files,
            selected: 0,
            file_offset: 0,
            diff: None,
            scroll: 0,
            horizontal: 0,
            page_height: 0,
            max_scroll: 0,
            max_horizontal: 0,
            content_width: 0,
        }
    }
}
