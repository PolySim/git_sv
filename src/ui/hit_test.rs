//! Hit-testing pour la navigation souris.
//!
//! Ce module fournit des fonctions pour déterminer dans quelle zone
//! de l'interface un clic souris a eu lieu, sans dupliquer la logique
//! de layout du rendu.

use ratatui::layout::Rect;

use crate::state::{AppState, ViewMode};
use crate::ui::branches_layout::build_branches_layout;
use crate::ui::layout::build_layout_with_diff_mode;
use crate::ui::project_tree_layout::build_project_tree_layout;
use crate::ui::staging_layout::build_staging_layout;

/// Zone cliquable détectée par le hit-testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClickableZone {
    /// Zone de navigation (onglets 1 à 5)
    NavBar,
    /// Zone du graphe de commits
    Graph,
    /// Zone du panneau bas-gauche (fichiers/status)
    BottomLeft,
    /// Zone du panneau bas-droit (détail/diff)
    BottomRight,
    /// Zone de la barre de recherche
    SearchBar,
    /// Zone de la barre d'aide
    HelpBar,
    /// Zone de la status bar
    StatusBar,
    /// Zone d'un popup modal (merge picker, reset picker, confirmation)
    Modal,
    /// Zone en dehors des éléments interactifs
    Outside,
}

/// Résultat d'un hit-test avec la position relative dans la zone.
#[derive(Debug, Clone)]
pub struct HitTestResult {
    /// Zone cliquable détectée
    pub zone: ClickableZone,
    /// Position X relative à la zone (0 = gauche)
    pub relative_x: u16,
    /// Position Y relative à la zone (0 = haut)
    pub relative_y: u16,
    /// Rectangle complet de la zone
    pub rect: Rect,
}

/// Détermine dans quelle zone cliquable se trouve la position (x, y).
///
/// Cette fonction calcule le layout et retourne la zone correspondante
/// à la position donnée.
pub fn hit_test(state: &AppState, x: u16, y: u16) -> Option<HitTestResult> {
    // Vérifier d'abord les modals et popups qui ont la priorité
    if let Some(zone) = hit_test_modals(state, x, y) {
        return Some(zone);
    }

    // Calculer le layout actuel à partir de la dernière zone réellement rendue.
    let screen_rect = state.screen_area;
    if screen_rect.width == 0 || screen_rect.height == 0 {
        return None;
    }

    if state.view_mode == ViewMode::Staging {
        return hit_test_staging(state, x, y, screen_rect);
    }

    if state.view_mode == ViewMode::Branches {
        return hit_test_branches(state, x, y, screen_rect);
    }

    if state.view_mode == ViewMode::ProjectTree {
        return hit_test_project_tree(state, x, y, screen_rect);
    }

    let layout = build_layout_with_diff_mode(
        screen_rect,
        state.search_state.is_active,
        state.graph_view.diff_fullscreen,
    );

    // Vérifier chaque zone en ordre de priorité
    if point_in_rect(x, y, layout.status_bar) {
        return Some(HitTestResult {
            zone: ClickableZone::StatusBar,
            relative_x: x - layout.status_bar.x,
            relative_y: y - layout.status_bar.y,
            rect: layout.status_bar,
        });
    }

    if point_in_rect(x, y, layout.nav_bar) {
        return Some(HitTestResult {
            zone: ClickableZone::NavBar,
            relative_x: x - layout.nav_bar.x,
            relative_y: y - layout.nav_bar.y,
            rect: layout.nav_bar,
        });
    }

    // Si mode diff plein écran
    if let Some(diff_rect) = layout.diff_fullscreen {
        if point_in_rect(x, y, diff_rect) {
            return Some(HitTestResult {
                zone: ClickableZone::BottomRight,
                relative_x: x - diff_rect.x,
                relative_y: y - diff_rect.y,
                rect: diff_rect,
            });
        }
    } else {
        // Mode normal avec graphe + panneaux bas
        if point_in_rect(x, y, layout.graph) {
            return Some(HitTestResult {
                zone: ClickableZone::Graph,
                relative_x: x - layout.graph.x,
                relative_y: y - layout.graph.y,
                rect: layout.graph,
            });
        }

        if point_in_rect(x, y, layout.bottom_left) {
            return Some(HitTestResult {
                zone: ClickableZone::BottomLeft,
                relative_x: x - layout.bottom_left.x,
                relative_y: y - layout.bottom_left.y,
                rect: layout.bottom_left,
            });
        }

        if point_in_rect(x, y, layout.bottom_right) {
            return Some(HitTestResult {
                zone: ClickableZone::BottomRight,
                relative_x: x - layout.bottom_right.x,
                relative_y: y - layout.bottom_right.y,
                rect: layout.bottom_right,
            });
        }
    }

    if let Some(search_rect) = layout.search_bar {
        if point_in_rect(x, y, search_rect) {
            return Some(HitTestResult {
                zone: ClickableZone::SearchBar,
                relative_x: x - search_rect.x,
                relative_y: y - search_rect.y,
                rect: search_rect,
            });
        }
    }

    if point_in_rect(x, y, layout.help_bar) {
        return Some(HitTestResult {
            zone: ClickableZone::HelpBar,
            relative_x: x - layout.help_bar.x,
            relative_y: y - layout.help_bar.y,
            rect: layout.help_bar,
        });
    }

    Some(HitTestResult {
        zone: ClickableZone::Outside,
        relative_x: x,
        relative_y: y,
        rect: screen_rect,
    })
}

fn hit_test_project_tree(
    state: &AppState,
    x: u16,
    y: u16,
    screen_rect: Rect,
) -> Option<HitTestResult> {
    let layout = build_project_tree_layout(screen_rect, state.project_tree_state.search.is_active);

    for (zone, rect) in [
        (ClickableZone::StatusBar, layout.status_bar),
        (ClickableZone::NavBar, layout.nav_bar),
        (
            ClickableZone::SearchBar,
            layout.search_bar.unwrap_or_default(),
        ),
        (ClickableZone::BottomLeft, layout.tree_panel),
        (ClickableZone::BottomRight, layout.history_panel),
        (ClickableZone::BottomLeft, layout.changed_files_panel),
        (ClickableZone::BottomRight, layout.diff_panel),
        (ClickableZone::HelpBar, layout.help_bar),
    ] {
        if point_in_rect(x, y, rect) {
            return Some(HitTestResult {
                zone,
                relative_x: x - rect.x,
                relative_y: y - rect.y,
                rect,
            });
        }
    }

    Some(HitTestResult {
        zone: ClickableZone::Outside,
        relative_x: x,
        relative_y: y,
        rect: screen_rect,
    })
}

fn hit_test_staging(_state: &AppState, x: u16, y: u16, screen_rect: Rect) -> Option<HitTestResult> {
    let layout = build_staging_layout(screen_rect);

    for (zone, rect) in [
        (ClickableZone::StatusBar, layout.status_bar),
        (ClickableZone::NavBar, layout.nav_bar),
        (ClickableZone::BottomLeft, layout.unstaged_panel),
        (ClickableZone::BottomLeft, layout.staged_panel),
        (ClickableZone::BottomRight, layout.diff_panel),
        (ClickableZone::SearchBar, layout.commit_message),
        (ClickableZone::HelpBar, layout.help_bar),
    ] {
        if point_in_rect(x, y, rect) {
            return Some(HitTestResult {
                zone,
                relative_x: x - rect.x,
                relative_y: y - rect.y,
                rect,
            });
        }
    }

    Some(HitTestResult {
        zone: ClickableZone::Outside,
        relative_x: x,
        relative_y: y,
        rect: screen_rect,
    })
}

fn hit_test_branches(
    _state: &AppState,
    x: u16,
    y: u16,
    screen_rect: Rect,
) -> Option<HitTestResult> {
    let layout = build_branches_layout(screen_rect);

    for (zone, rect) in [
        (ClickableZone::StatusBar, layout.status_bar),
        (ClickableZone::NavBar, layout.nav_bar),
        (ClickableZone::NavBar, layout.tabs),
        (ClickableZone::BottomLeft, layout.list_panel),
        (ClickableZone::BottomRight, layout.detail_panel),
        (ClickableZone::HelpBar, layout.help_bar),
    ] {
        if point_in_rect(x, y, rect) {
            return Some(HitTestResult {
                zone,
                relative_x: x - rect.x,
                relative_y: y - rect.y,
                rect,
            });
        }
    }

    Some(HitTestResult {
        zone: ClickableZone::Outside,
        relative_x: x,
        relative_y: y,
        rect: screen_rect,
    })
}

/// Vérifie si un point est dans un rectangle.
fn point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// Vérifie si un point est dans un modal ou popup.
fn hit_test_modals(state: &AppState, x: u16, y: u16) -> Option<HitTestResult> {
    // Merge picker
    if let Some(ref picker) = state.merge_picker {
        if picker.is_active {
            // Le merge picker est centré, on considère toute l'écran comme modal
            // pour bloquer les interactions derrière
            return Some(HitTestResult {
                zone: ClickableZone::Modal,
                relative_x: x,
                relative_y: y,
                rect: Rect::new(0, 0, u16::MAX, u16::MAX),
            });
        }
    }

    // Reset picker
    if let Some(ref picker) = state.reset_picker {
        if picker.is_active {
            return Some(HitTestResult {
                zone: ClickableZone::Modal,
                relative_x: x,
                relative_y: y,
                rect: Rect::new(0, 0, u16::MAX, u16::MAX),
            });
        }
    }

    // Confirmation dialog
    if state.ui.pending_confirmation.is_some() {
        return Some(HitTestResult {
            zone: ClickableZone::Modal,
            relative_x: x,
            relative_y: y,
            rect: Rect::new(0, 0, u16::MAX, u16::MAX),
        });
    }

    // Filter popup
    if state.filters.filter_popup.is_open {
        return Some(HitTestResult {
            zone: ClickableZone::Modal,
            relative_x: x,
            relative_y: y,
            rect: Rect::new(0, 0, u16::MAX, u16::MAX),
        });
    }

    None
}

/// Calcule l'index d'un commit dans le graphe à partir d'une position Y.
///
/// Retourne None si la position est en dehors de la liste visible.
pub fn calculate_commit_index(
    graph_height: usize,
    visible_offset: usize,
    relative_y: u16,
) -> Option<usize> {
    // Chaque commit prend environ 2 lignes (commit + connexion)
    const LINES_PER_COMMIT: u16 = 2;

    let line_index = relative_y / LINES_PER_COMMIT;
    let commit_index = visible_offset + line_index as usize;

    if commit_index < graph_height {
        Some(commit_index)
    } else {
        None
    }
}

/// Calcule l'index d'un fichier dans la liste à partir d'une position Y.
pub fn calculate_file_index(file_count: usize, relative_y: u16) -> Option<usize> {
    // Les fichiers prennent 1 ligne chacun
    let index = relative_y as usize;

    if index < file_count {
        Some(index)
    } else {
        None
    }
}

/// Détermine quel tab de navigation est cliqué.
pub fn calculate_nav_tab(relative_x: u16, unresolved_conflicts: usize) -> Option<ViewMode> {
    use crate::i18n::text;

    let tabs = [
        (text("Graphe", "Graph"), ViewMode::Graph),
        (text("Staging", "Staging"), ViewMode::Staging),
        (text("Branches", "Branches"), ViewMode::Branches),
        (text("Arbre", "Tree"), ViewMode::ProjectTree),
    ];
    let mut offset = 1usize;
    let x = usize::from(relative_x);

    for (label, view) in tabs {
        let width = 3 + label.chars().count() + 2;
        if x >= offset && x < offset + width {
            return Some(view);
        }
        offset += width;
    }

    if unresolved_conflicts > 0 && x >= offset {
        Some(ViewMode::Conflicts)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_in_rect() {
        let rect = Rect::new(10, 10, 20, 10);

        assert!(point_in_rect(10, 10, rect)); // Coin supérieur gauche
        assert!(point_in_rect(15, 15, rect)); // Centre
        assert!(point_in_rect(29, 19, rect)); // Juste avant coin inférieur droit

        assert!(!point_in_rect(9, 15, rect)); // À gauche
        assert!(!point_in_rect(30, 15, rect)); // À droite
        assert!(!point_in_rect(15, 9, rect)); // Au-dessus
        assert!(!point_in_rect(15, 20, rect)); // En-dessous
    }

    #[test]
    fn test_calculate_commit_index() {
        // Avec un offset de 10 et une hauteur de 100 commits
        assert_eq!(calculate_commit_index(100, 10, 0), Some(10));
        assert_eq!(calculate_commit_index(100, 10, 1), Some(10)); // Même commit (ligne de connexion)
        assert_eq!(calculate_commit_index(100, 10, 2), Some(11));
        assert_eq!(calculate_commit_index(100, 10, 4), Some(12));

        // Hors limites
        assert_eq!(calculate_commit_index(100, 10, 200), None);
    }

    #[test]
    fn test_calculate_file_index() {
        assert_eq!(calculate_file_index(10, 0), Some(0));
        assert_eq!(calculate_file_index(10, 5), Some(5));
        assert_eq!(calculate_file_index(10, 9), Some(9));
        assert_eq!(calculate_file_index(10, 10), None); // Hors limites
    }

    #[test]
    fn test_calculate_nav_tab() {
        assert_eq!(calculate_nav_tab(1, 0), Some(ViewMode::Graph));
        assert_eq!(calculate_nav_tab(10, 0), Some(ViewMode::Graph));
        assert_eq!(calculate_nav_tab(15, 0), Some(ViewMode::Staging));
        assert_eq!(calculate_nav_tab(30, 0), Some(ViewMode::Branches));
        assert_eq!(calculate_nav_tab(40, 0), Some(ViewMode::ProjectTree));
        assert_eq!(calculate_nav_tab(50, 2), Some(ViewMode::Conflicts));
        assert_eq!(calculate_nav_tab(100, 0), None); // Hors limites
    }

    #[test]
    fn test_outside_returns_outside_zone() {
        use crate::git::repo::GitRepo;
        use crate::git::tests::test_utils::create_test_repo;

        let (temp_dir, _repo) = create_test_repo();
        let git_repo = GitRepo::open(temp_dir.path().to_string_lossy().as_ref()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
        state.screen_area = Rect::new(0, 0, 120, 40);

        let result = hit_test(&state, 119, 39).expect("hit test devrait répondre");
        assert!(matches!(
            result.zone,
            ClickableZone::HelpBar | ClickableZone::Outside
        ));
    }
}
