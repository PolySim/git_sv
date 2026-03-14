use git2::Oid;

/// Type de référence git.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Head,
}

/// Information sur une référence git.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefInfo {
    pub name: String,
    pub ref_type: RefType,
}

impl RefInfo {
    pub fn new(name: impl Into<String>, ref_type: RefType) -> Self {
        Self {
            name: name.into(),
            ref_type,
        }
    }
}

/// Type de segment visuel dans le graphe.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Vertical,
    ForkRight,
    ForkLeft,
    MergeFromRight,
    MergeFromLeft,
    Horizontal,
    Cross,
}

/// Cellule du graphe : représente ce qui est dessiné dans une colonne donnée.
#[derive(Debug, Clone)]
pub struct GraphCell {
    pub edge_type: EdgeType,
    pub color_index: usize,
}

/// Rangée intermédiaire entre deux commits (pour les connexions).
#[derive(Debug, Clone)]
pub struct ConnectionRow {
    pub cells: Vec<Option<GraphCell>>,
}

/// Noeud du graphe de commits, enrichi avec des infos de placement.
#[derive(Debug, Clone)]
pub struct CommitNode {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
    pub parents: Vec<Oid>,
    pub refs: Vec<RefInfo>,
    pub branch_name: Option<String>,
    pub column: usize,
    pub color_index: usize,
}

impl Default for CommitNode {
    fn default() -> Self {
        Self {
            oid: Oid::zero(),
            message: String::new(),
            author: String::new(),
            timestamp: 0,
            parents: Vec::new(),
            refs: Vec::new(),
            branch_name: None,
            column: 0,
            color_index: 0,
        }
    }
}

impl CommitNode {
    pub fn short_hash(&self) -> String {
        self.oid.to_string()[..7].to_string()
    }
}

/// Rangée du graphe contenant le commit et les segments de connexion.
#[derive(Debug, Clone, Default)]
pub struct GraphRow {
    pub node: CommitNode,
    pub cells: Vec<Option<GraphCell>>,
    pub connection: Option<ConnectionRow>,
}

/// État d'une colonne active pendant la construction du graphe.
#[derive(Debug, Clone)]
pub(super) struct ColumnState {
    pub expected_oid: Option<Oid>,
    pub color_index: usize,
    pub branch_name: Option<String>,
}
