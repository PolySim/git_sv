//! Handler pour les actions d'édition de texte.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::EditAction;
use crate::state::{AppState, BranchesFocus, ViewMode};

/// Buffer de texte éditable avec curseur.
struct TextBuffer<'a> {
    text: &'a mut String,
    cursor: &'a mut usize,
}

impl<'a> TextBuffer<'a> {
    /// Crée un nouveau buffer.
    fn new(text: &'a mut String, cursor: &'a mut usize) -> Self {
        Self { text, cursor }
    }

    /// Insère un caractère à la position du curseur.
    fn insert_char(&mut self, c: char) {
        if *self.cursor <= self.text.len() {
            self.text.insert(*self.cursor, c);
            *self.cursor += 1;
        }
    }

    /// Supprime le caractère avant le curseur.
    fn delete_char_before(&mut self) {
        if *self.cursor > 0 && *self.cursor <= self.text.len() {
            self.text.remove(*self.cursor - 1);
            *self.cursor -= 1;
        }
    }

    /// Supprime le caractère après le curseur.
    fn delete_char_after(&mut self) {
        if *self.cursor < self.text.len() {
            self.text.remove(*self.cursor);
        }
    }

    /// Déplace le curseur à gauche.
    fn cursor_left(&mut self) {
        if *self.cursor > 0 {
            *self.cursor -= 1;
        }
    }

    /// Déplace le curseur à droite.
    fn cursor_right(&mut self) {
        if *self.cursor < self.text.len() {
            *self.cursor += 1;
        }
    }

    /// Déplace le curseur au début.
    fn cursor_home(&mut self) {
        *self.cursor = 0;
    }

    /// Déplace le curseur à la fin.
    fn cursor_end(&mut self) {
        *self.cursor = self.text.len();
    }

    /// Insère une nouvelle ligne.
    fn insert_newline(&mut self) {
        if *self.cursor <= self.text.len() {
            self.text.insert(*self.cursor, '\n');
            *self.cursor += 1;
        }
    }
}

/// Handler pour les opérations d'édition de texte.
pub struct EditHandler;

impl ActionHandler for EditHandler {
    type Action = EditAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: EditAction) -> Result<()> {
        // Déterminer quel buffer est actif selon le contexte
        let mut buffer = get_active_buffer(ctx.state);

        match action {
            EditAction::InsertChar(c) => buffer.insert_char(c),
            EditAction::DeleteCharBefore => buffer.delete_char_before(),
            EditAction::DeleteCharAfter => buffer.delete_char_after(),
            EditAction::CursorLeft => buffer.cursor_left(),
            EditAction::CursorRight => buffer.cursor_right(),
            EditAction::CursorHome => buffer.cursor_home(),
            EditAction::CursorEnd => buffer.cursor_end(),
            EditAction::NewLine => buffer.insert_newline(),
        }

        Ok(())
    }
}

/// Détermine le buffer actif selon le contexte de l'application.
fn get_active_buffer(state: &mut AppState) -> TextBuffer<'_> {
    // Priorité 1 : Vue Branches en mode Input
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.focus == BranchesFocus::Input
    {
        return TextBuffer::new(
            &mut state.branches_view_state.input_text,
            &mut state.branches_view_state.input_cursor,
        );
    }

    // Priorité 2 : Vue Staging en mode commit
    if state.staging_state.is_committing {
        return TextBuffer::new(
            &mut state.staging_state.commit_message,
            &mut state.staging_state.cursor_position,
        );
    }

    // Fallback : ne rien modifier (buffer vide)
    // Ceci ne devrait pas arriver en conditions normales car les handlers
    // d'édition ne sont activés que dans les modes appropriés
    panic!("Aucun buffer actif trouvé pour l'édition");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::InputAction;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(dir.path(), &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(dir.path().to_str().unwrap()).unwrap();
        (dir, git_repo)
    }

    #[test]
    fn test_text_buffer_insert_char() {
        let mut text = String::new();
        let mut cursor = 0;
        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.insert_char('a');
        }
        assert_eq!(text, "a");
        assert_eq!(cursor, 1);

        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.insert_char('b');
        }
        assert_eq!(text, "ab");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_text_buffer_delete_char_before() {
        let mut text = "abc".to_string();
        let mut cursor = 2;
        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.delete_char_before();
        }
        assert_eq!(text, "ac");
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_text_buffer_cursor_movement() {
        let mut text = "hello".to_string();
        let mut cursor = 2;

        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.cursor_home();
        }
        assert_eq!(cursor, 0);

        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.cursor_end();
        }
        assert_eq!(cursor, 5);

        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.cursor_left();
        }
        assert_eq!(cursor, 4);

        {
            let mut buffer = TextBuffer::new(&mut text, &mut cursor);
            buffer.cursor_right();
        }
        assert_eq!(cursor, 5);
    }

    #[test]
    fn test_edit_handler_in_branches_context() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();

        // Configurer le contexte Branches Input
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;

        let mut handler = EditHandler;

        // Insérer des caractères
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, EditAction::InsertChar('t'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('e'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('s'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('t'))
                .unwrap();
        }

        assert_eq!(state.branches_view_state.input_text, "test");
        assert_eq!(state.branches_view_state.input_cursor, 4);

        // Déplacer le curseur à gauche
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, EditAction::CursorLeft).unwrap();
        }
        assert_eq!(state.branches_view_state.input_cursor, 3);

        // Supprimer le caractère avant (supprime le 's')
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, EditAction::DeleteCharBefore)
                .unwrap();
        }
        assert_eq!(state.branches_view_state.input_text, "tet");
        assert_eq!(state.branches_view_state.input_cursor, 2);
    }

    #[test]
    fn test_edit_handler_in_staging_context() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();

        // Configurer le contexte Staging Commit
        state.view_mode = ViewMode::Staging;
        state.staging_state.is_committing = true;
        state.staging_state.commit_message.clear();
        state.staging_state.cursor_position = 0;

        let mut handler = EditHandler;

        // Insérer des caractères
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, EditAction::InsertChar('f'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('i'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('x'))
                .unwrap();
        }

        assert_eq!(state.staging_state.commit_message, "fix");
        assert_eq!(state.staging_state.cursor_position, 3);

        // Insérer une nouvelle ligne
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, EditAction::NewLine).unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('b'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('u'))
                .unwrap();
            handler
                .handle(&mut ctx, EditAction::InsertChar('g'))
                .unwrap();
        }

        assert_eq!(state.staging_state.commit_message, "fix\nbug");
        assert_eq!(state.staging_state.cursor_position, 7);
    }

    #[test]
    fn test_branches_context_priority_over_staging() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();

        // Configurer les DEUX contextes actifs
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.staging_state.is_committing = true;

        // Initialiser les buffers
        state.branches_view_state.input_text = "branch-text".to_string();
        state.branches_view_state.input_cursor = 0;
        state.staging_state.commit_message = "commit-text".to_string();
        state.staging_state.cursor_position = 0;

        let mut handler = EditHandler;

        // Insérer un caractère - devrait aller dans le buffer Branches
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, EditAction::InsertChar('X'))
                .unwrap();
        }

        // Le buffer Branches doit être modifié
        assert_eq!(state.branches_view_state.input_text, "Xbranch-text");
        // Le buffer Staging ne doit PAS être modifié
        assert_eq!(state.staging_state.commit_message, "commit-text");
    }
}
