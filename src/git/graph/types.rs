use std::num::NonZeroU8;

use git2::Oid;

/// Nombre de couleurs distinctes utilisées pour les branches.
pub const GRAPH_COLOR_COUNT: usize = 12;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Vertical,
    ForkRight,
    ForkLeft,
    MergeFromRight,
    MergeFromLeft,
    Horizontal,
    Cross,
}

/// Cellule du graphe compactée sur un octet.
///
/// La valeur zéro reste libre afin que `Option<GraphCell>` occupe également
/// un seul octet. Les autres valeurs encodent le type d'arête et la couleur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphCell(NonZeroU8);

impl GraphCell {
    const EDGE_TYPE_COUNT: usize = 7;

    pub fn new(edge_type: EdgeType, color_index: usize) -> Self {
        let edge_code = match edge_type {
            EdgeType::Vertical => 0,
            EdgeType::ForkRight => 1,
            EdgeType::ForkLeft => 2,
            EdgeType::MergeFromRight => 3,
            EdgeType::MergeFromLeft => 4,
            EdgeType::Horizontal => 5,
            EdgeType::Cross => 6,
        };
        let normalized_color = color_index % GRAPH_COLOR_COUNT;
        let packed = normalized_color * Self::EDGE_TYPE_COUNT + edge_code + 1;

        Self(NonZeroU8::new(packed as u8).expect("une cellule encodée ne peut pas valoir zéro"))
    }

    pub fn edge_type(self) -> EdgeType {
        match (usize::from(self.0.get()) - 1) % Self::EDGE_TYPE_COUNT {
            0 => EdgeType::Vertical,
            1 => EdgeType::ForkRight,
            2 => EdgeType::ForkLeft,
            3 => EdgeType::MergeFromRight,
            4 => EdgeType::MergeFromLeft,
            5 => EdgeType::Horizontal,
            6 => EdgeType::Cross,
            _ => unreachable!("le code d'arête est borné par le modulo"),
        }
    }

    pub fn color_index(self) -> usize {
        (usize::from(self.0.get()) - 1) / Self::EDGE_TYPE_COUNT
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_cell_round_trip() {
        let edge_types = [
            EdgeType::Vertical,
            EdgeType::ForkRight,
            EdgeType::ForkLeft,
            EdgeType::MergeFromRight,
            EdgeType::MergeFromLeft,
            EdgeType::Horizontal,
            EdgeType::Cross,
        ];

        for edge_type in edge_types {
            for color_index in 0..(GRAPH_COLOR_COUNT * 2) {
                let cell = GraphCell::new(edge_type, color_index);
                assert_eq!(cell.edge_type(), edge_type);
                assert_eq!(cell.color_index(), color_index % GRAPH_COLOR_COUNT);
            }
        }
    }

    #[test]
    fn test_optional_graph_cell_fits_in_one_byte() {
        assert_eq!(std::mem::size_of::<Option<GraphCell>>(), 1);
    }
}
