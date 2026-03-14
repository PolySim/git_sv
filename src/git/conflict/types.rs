/// Source d'une ligne dans le resultat resolu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSource {
    Context,
    Ours,
    Theirs,
    ConflictMarker,
}

/// Ligne resolue avec sa provenance.
#[derive(Debug, Clone)]
pub struct ResolvedLine {
    pub content: String,
    pub source: LineSource,
}

/// Mode de resolution des conflits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolutionMode {
    File,
    Block,
    Line,
}

/// Type de conflit sur un fichier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictType {
    BothModified,
    DeletedByUs,
    DeletedByThem,
    BothAdded,
}

/// Resolution au niveau ligne - permet de choisir individuellement quelles lignes inclure.
#[derive(Debug, Clone, PartialEq)]
pub struct LineLevelResolution {
    pub ours_lines_included: Vec<bool>,
    pub theirs_lines_included: Vec<bool>,
    pub touched: bool,
}

impl LineLevelResolution {
    /// Cree une nouvelle resolution ligne par ligne avec toutes les lignes ours incluses par defaut.
    pub fn new(ours_count: usize, theirs_count: usize) -> Self {
        Self {
            ours_lines_included: vec![true; ours_count],
            theirs_lines_included: vec![false; theirs_count],
            touched: false,
        }
    }
}

/// Resolution possible pour une section de conflit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    Both,
}

/// Section de conflit enrichie.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictSection {
    pub context_before: Vec<String>,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
    pub context_after: Vec<String>,
    pub resolution: Option<ConflictResolution>,
    pub line_level_resolution: Option<LineLevelResolution>,
}

/// Un fichier en conflit.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictFile {
    pub path: String,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
    pub conflict_type: ConflictType,
}

/// Fichier dans un merge (en conflit ou non).
#[derive(Debug, Clone)]
pub struct MergeFile {
    pub path: String,
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
    pub conflict_type: Option<ConflictType>,
}

/// Resultat d'une operation de merge.
#[derive(Debug)]
pub enum MergeResult {
    Success,
    FastForward,
    UpToDate,
    Conflicts(Vec<ConflictFile>),
}
