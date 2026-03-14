//! Définition centralisée des raccourcis clavier.
//!
//! Ce module contient toutes les définitions de touches utilisées dans l'application
//! pour éviter la divergence entre la documentation et le code.

#![allow(dead_code)]

/// Navigation dans le graphe de commits.
pub mod navigation {
    /// Commit suivant.
    pub const DOWN: &[&str] = &["j", "↓"];
    /// Commit précédent.
    pub const UP: &[&str] = &["k", "↑"];
    /// Premier commit.
    pub const TOP: &[&str] = &["g", "Home"];
    /// Dernier commit.
    pub const BOTTOM: &[&str] = &["G", "End"];
    /// Page suivante.
    pub const PAGE_DOWN: &[&str] = &["Ctrl+D", "PgDn"];
    /// Page précédente.
    pub const PAGE_UP: &[&str] = &["Ctrl+U", "PgUp"];
    /// Bascule entre panneaux.
    pub const SWITCH_PANEL: &str = "Tab";
}

/// Actions globales disponibles dans toutes les vues.
pub mod global {
    /// Quitter l'application.
    pub const QUIT: &[&str] = &["q", "Ctrl+C"];
    /// Afficher l'aide.
    pub const HELP: &str = "?";
    /// Rafraîchir les données.
    pub const REFRESH: &str = "r";
    /// Copier dans le presse-papiers.
    pub const COPY: &str = "y";

    /// Vue Graph (historique).
    pub const VIEW_GRAPH: &str = "1";
    /// Vue Staging.
    pub const VIEW_STAGING: &str = "2";
    /// Vue Branches.
    pub const VIEW_BRANCHES: &str = "3";
    /// Vue Conflits (si actifs).
    pub const VIEW_CONFLICTS: &str = "4";
}

/// Actions Git disponibles dans la vue Graph.
pub mod git_actions {
    /// Push.
    pub const PUSH: &str = "P";
    /// Pull.
    pub const PULL: &str = "p";
    /// Force push.
    pub const FORCE_PUSH: &str = "Ctrl+P";
    /// Fetch.
    pub const FETCH: &str = "f";
    /// Ouvrir la vue branches.
    pub const BRANCHES: &str = "b";
    /// Nouveau commit.
    pub const COMMIT: &str = "c";
    /// Stash rapide.
    pub const STASH: &str = "s";
    /// Merge.
    pub const MERGE: &str = "m";
    /// Cherry-pick.
    pub const CHERRY_PICK: &str = "x";
    /// Git blame.
    pub const BLAME: &str = "B";
    /// Reset.
    pub const RESET: &str = "R";
    /// Annuler le merge (si en cours).
    pub const ABORT_MERGE: &str = "A";
    /// Charger plus d'historique.
    pub const LOAD_MORE: &str = "L";
}

/// Recherche et filtres.
pub mod search {
    /// Ouvrir la recherche.
    pub const OPEN: &str = "/";
    /// Résultat suivant.
    pub const NEXT: &str = "n";
    /// Résultat précédent.
    pub const PREVIOUS: &str = "N";
    /// Filtre avancé.
    pub const FILTER: &str = "F";
}

/// Vue Branches.
pub mod branches {
    /// Checkout la branche sélectionnée.
    pub const CHECKOUT: &str = "Enter";
    /// Créer une nouvelle branche.
    pub const NEW: &str = "n";
    /// Supprimer la branche.
    pub const DELETE: &str = "d";
    /// Renommer la branche.
    pub const RENAME: &str = "r";
    /// Basculer l'affichage des branches distantes.
    pub const TOGGLE_REMOTE: &str = "R";
    /// Merge.
    pub const MERGE: &str = "m";
    /// Section suivante.
    pub const NEXT_SECTION: &str = "Tab";
    /// Section précédente.
    pub const PREV_SECTION: &str = "Shift+Tab";

    /// Créer un worktree.
    pub const WORKTREE_NEW: &str = "n";
    /// Supprimer un worktree.
    pub const WORKTREE_DELETE: &str = "d";

    /// Sauvegarder un stash.
    pub const STASH_SAVE: &str = "s";
    /// Appliquer un stash.
    pub const STASH_APPLY: &str = "a";
    /// Pop un stash.
    pub const STASH_POP: &str = "p";
    /// Supprimer un stash.
    pub const STASH_DROP: &str = "d";
    /// Fichier suivant dans le stash.
    pub const STASH_FILE_NEXT: &str = "l";
    /// Fichier précédent dans le stash.
    pub const STASH_FILE_PREV: &str = "h";
}

/// Vue Staging.
pub mod staging {
    /// Stage le fichier sélectionné.
    pub const STAGE: &[&str] = &["s", "Enter"];
    /// Stage tous les fichiers.
    pub const STAGE_ALL: &str = "a";
    /// Stash le fichier sélectionné.
    pub const STASH_FILE: &str = "S";
    /// Stash tous les fichiers unstaged.
    pub const STASH_ALL: &str = "Ctrl+S";
    /// Unstage le fichier sélectionné.
    pub const UNSTAGE: &[&str] = &["u", "Enter"];
    /// Unstage tous les fichiers.
    pub const UNSTAGE_ALL: &str = "U";
    /// Discard le fichier.
    pub const DISCARD: &str = "d";
    /// Discard tous les fichiers.
    pub const DISCARD_ALL: &str = "D";
    /// Commencer la saisie du message de commit.
    pub const COMMIT_MESSAGE: &str = "c";
    /// Amend commit.
    pub const AMEND: &str = "A";
    /// Basculer le focus.
    pub const SWITCH_FOCUS: &[&str] = &["Tab", "Space"];
}

/// Diff et affichage.
pub mod diff {
    /// Basculer mode diff (unified/split).
    pub const TOGGLE_VIEW: &str = "v";
    /// Plein écran.
    pub const FULLSCREEN: &[&str] = &["z", "Enter"];
    /// Scroll vers le haut.
    pub const UP: &[&str] = &["k", "↑"];
    /// Scroll vers le bas.
    pub const DOWN: &[&str] = &["j", "↓"];
    /// Scroll à gauche.
    pub const LEFT: &[&str] = &["h", "←"];
    /// Scroll à droite.
    pub const RIGHT: &[&str] = &["l", "→"];
}

/// Vue Blame.
pub mod blame {
    /// Fermer la vue blame.
    pub const CLOSE: &[&str] = &["q", "Esc"];
    /// Sauter au commit.
    pub const JUMP: &str = "Enter";
}

/// Résolution de conflits.
pub mod conflicts {
    /// Basculer entre panneaux.
    pub const SWITCH_PANEL: &[&str] = &["Tab", "Shift+Tab"];
    /// Accepter 'ours'.
    pub const ACCEPT_OURS: &[&str] = &["o", "←"];
    /// Accepter 'theirs'.
    pub const ACCEPT_THEIRS: &[&str] = &["t", "→"];
    /// Accepter les deux (mode bloc).
    pub const ACCEPT_BOTH: &str = "b";
    /// Marquer comme résolu.
    pub const MARK_RESOLVED: &str = "Enter";
    /// Finaliser le merge.
    pub const FINALIZE: &str = "V";
    /// Annuler le merge.
    pub const ABORT: &str = "A";
    /// Mode fichier.
    pub const MODE_FILE: &str = "F";
    /// Mode bloc.
    pub const MODE_BLOCK: &str = "B";
    /// Mode ligne.
    pub const MODE_LINE: &str = "L";
    /// Éditer le résultat.
    pub const EDIT: &[&str] = &["i", "e"];
}

/// Formatte une liste de touches pour l'affichage.
/// Ex: ["j", "↓"] → "j / ↓"
pub fn format_keys(keys: &[&str]) -> String {
    keys.join(" / ")
}

/// Formatte une description avec ses touches.
/// Ex: ("Commit suivant", ["j", "↓"]) → "j / ↓    Commit suivant"
pub fn format_key_line(description: &str, keys: &[&str]) -> String {
    let keys_str = format_keys(keys);
    let padding = 16usize.saturating_sub(keys_str.len());
    format!("{}{}{}", keys_str, " ".repeat(padding), description)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_keys() {
        assert_eq!(format_keys(&["j", "↓"]), "j / ↓");
        assert_eq!(format_keys(&["Enter"]), "Enter");
    }

    #[test]
    fn test_format_key_line() {
        let line = format_key_line("Commit suivant", &["j", "↓"]);
        assert!(line.contains("j / ↓"));
        assert!(line.contains("Commit suivant"));
    }

    #[test]
    fn test_all_keybindings_defined() {
        // Vérifier que toutes les constantes essentielles sont définies
        assert!(!global::QUIT.is_empty());
        assert!(!git_actions::PUSH.is_empty());
        assert!(!git_actions::PULL.is_empty());
        assert!(!branches::CHECKOUT.is_empty());
    }
}
