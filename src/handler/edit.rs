//! Handler pour les actions d'édition de texte.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::EditAction;
use crate::state::text_edit::TextSnapshot;
use crate::state::{selection_range, AppState, BranchesFocus, TextEditHistory, ViewMode};

/// Buffer de texte éditable avec curseur.
struct TextBuffer<'a> {
    text: &'a mut String,
    cursor: &'a mut usize,
    selection_anchor: &'a mut Option<usize>,
    history: &'a mut TextEditHistory,
}

impl<'a> TextBuffer<'a> {
    /// Crée un nouveau buffer.
    fn new(
        text: &'a mut String,
        cursor: &'a mut usize,
        selection_anchor: &'a mut Option<usize>,
        history: &'a mut TextEditHistory,
    ) -> Self {
        *cursor = (*cursor).min(text.chars().count());
        Self {
            text,
            cursor,
            selection_anchor,
            history,
        }
    }

    /// Insère un caractère à la position du curseur.
    fn insert_char(&mut self, c: char) {
        self.record_snapshot();
        self.delete_selection();
        let byte_index = char_to_byte_index(self.text, *self.cursor);
        self.text.insert(byte_index, c);
        *self.cursor += 1;
    }

    /// Supprime le caractère avant le curseur.
    fn delete_char_before(&mut self) {
        if self.has_selection() {
            self.record_snapshot();
            self.delete_selection();
        } else if *self.cursor > 0 {
            self.record_snapshot();
            let start = *self.cursor - 1;
            self.remove_char_range(start, *self.cursor);
            *self.cursor = start;
        }
    }

    /// Supprime le caractère après le curseur.
    fn delete_char_after(&mut self) {
        if self.has_selection() {
            self.record_snapshot();
            self.delete_selection();
        } else if *self.cursor < self.char_count() {
            self.record_snapshot();
            self.remove_char_range(*self.cursor, *self.cursor + 1);
        }
    }

    /// Déplace le curseur à gauche.
    fn cursor_left(&mut self) {
        if let Some(range) = self.selection() {
            *self.cursor = range.start;
        } else {
            *self.cursor = self.cursor.saturating_sub(1);
        }
        *self.selection_anchor = None;
    }

    /// Déplace le curseur à droite.
    fn cursor_right(&mut self) {
        if let Some(range) = self.selection() {
            *self.cursor = range.end;
        } else {
            *self.cursor = (*self.cursor + 1).min(self.char_count());
        }
        *self.selection_anchor = None;
    }

    fn cursor_word_left(&mut self) {
        if let Some(range) = self.selection() {
            *self.cursor = range.start;
        } else {
            *self.cursor = previous_word_boundary(self.text, *self.cursor);
        }
        *self.selection_anchor = None;
    }

    fn cursor_word_right(&mut self) {
        if let Some(range) = self.selection() {
            *self.cursor = range.end;
        } else {
            *self.cursor = next_word_boundary(self.text, *self.cursor);
        }
        *self.selection_anchor = None;
    }

    /// Déplace le curseur au début.
    fn cursor_home(&mut self) {
        *self.cursor = line_start(self.text, *self.cursor);
        *self.selection_anchor = None;
    }

    /// Déplace le curseur à la fin.
    fn cursor_end(&mut self) {
        *self.cursor = line_end(self.text, *self.cursor);
        *self.selection_anchor = None;
    }

    /// Insère une nouvelle ligne.
    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    fn select_left(&mut self) {
        self.select_to(self.cursor.saturating_sub(1));
    }

    fn select_right(&mut self) {
        self.select_to((*self.cursor + 1).min(self.char_count()));
    }

    fn select_word_left(&mut self) {
        self.select_to(previous_word_boundary(self.text, *self.cursor));
    }

    fn select_word_right(&mut self) {
        self.select_to(next_word_boundary(self.text, *self.cursor));
    }

    fn select_to(&mut self, target: usize) {
        if self.selection_anchor.is_none() {
            *self.selection_anchor = Some(*self.cursor);
        }
        *self.cursor = target;
    }

    fn select_all(&mut self) {
        *self.selection_anchor = Some(0);
        *self.cursor = self.char_count();
    }

    fn select_home(&mut self) {
        self.select_to(line_start(self.text, *self.cursor));
    }

    fn select_end(&mut self) {
        self.select_to(line_end(self.text, *self.cursor));
    }

    fn delete_word_before(&mut self) {
        if self.has_selection() {
            self.record_snapshot();
            self.delete_selection();
            return;
        }

        let start = previous_word_boundary(self.text, *self.cursor);
        if start < *self.cursor {
            self.record_snapshot();
            self.remove_char_range(start, *self.cursor);
            *self.cursor = start;
        }
    }

    fn delete_to_start(&mut self) {
        if self.has_selection() {
            self.record_snapshot();
            self.delete_selection();
        } else {
            let start = line_start(self.text, *self.cursor);
            if start == *self.cursor {
                return;
            }
            self.record_snapshot();
            self.remove_char_range(start, *self.cursor);
            *self.cursor = start;
        }
    }

    fn delete_to_end(&mut self) {
        if self.has_selection() {
            self.record_snapshot();
            self.delete_selection();
        } else {
            let end = line_end(self.text, *self.cursor);
            if end == *self.cursor {
                return;
            }
            self.record_snapshot();
            self.remove_char_range(*self.cursor, end);
        }
    }

    fn undo(&mut self) {
        let Some(snapshot) = self.history.undo.pop() else {
            return;
        };
        let current = self.snapshot();
        self.history.redo.push(current);
        self.restore(snapshot);
    }

    fn redo(&mut self) {
        let Some(snapshot) = self.history.redo.pop() else {
            return;
        };
        let current = self.snapshot();
        self.history.undo.push(current);
        self.restore(snapshot);
    }

    fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn selection(&self) -> Option<std::ops::Range<usize>> {
        selection_range(*self.cursor, *self.selection_anchor)
    }

    fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    fn delete_selection(&mut self) {
        let Some(range) = self.selection() else {
            return;
        };
        self.remove_char_range(range.start, range.end);
        *self.cursor = range.start;
        *self.selection_anchor = None;
    }

    fn remove_char_range(&mut self, start: usize, end: usize) {
        let start_byte = char_to_byte_index(self.text, start);
        let end_byte = char_to_byte_index(self.text, end);
        self.text.replace_range(start_byte..end_byte, "");
    }

    fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            text: self.text.clone(),
            cursor: *self.cursor,
            selection_anchor: *self.selection_anchor,
        }
    }

    fn record_snapshot(&mut self) {
        self.history.record(self.snapshot());
    }

    fn restore(&mut self, snapshot: TextSnapshot) {
        *self.text = snapshot.text;
        *self.cursor = snapshot.cursor.min(self.char_count());
        *self.selection_anchor = snapshot.selection_anchor;
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn previous_word_boundary(text: &str, cursor: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut index = cursor.min(chars.len());
    while index > 0 && chars[index - 1].is_whitespace() {
        index -= 1;
    }
    if index == 0 {
        return 0;
    }

    let word = is_word_char(chars[index - 1]);
    while index > 0 && !chars[index - 1].is_whitespace() && is_word_char(chars[index - 1]) == word {
        index -= 1;
    }
    index
}

fn next_word_boundary(text: &str, cursor: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut index = cursor.min(chars.len());
    if index == chars.len() {
        return index;
    }

    let word = is_word_char(chars[index]);
    while index < chars.len() && !chars[index].is_whitespace() && is_word_char(chars[index]) == word
    {
        index += 1;
    }
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    index
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn line_start(text: &str, cursor: usize) -> usize {
    text.chars()
        .take(cursor)
        .enumerate()
        .filter_map(|(index, character)| (character == '\n').then_some(index + 1))
        .last()
        .unwrap_or(0)
}

fn line_end(text: &str, cursor: usize) -> usize {
    text.chars()
        .enumerate()
        .skip(cursor)
        .find_map(|(index, character)| (character == '\n').then_some(index))
        .unwrap_or_else(|| text.chars().count())
}

/// Handler pour les opérations d'édition de texte.
pub struct EditHandler;

impl ActionHandler for EditHandler {
    type Action = EditAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: EditAction) -> Result<()> {
        // Déterminer quel buffer est actif selon le contexte
        let Some(mut buffer) = get_active_buffer(ctx.state) else {
            return Ok(());
        };

        match action {
            EditAction::InsertChar(c) => buffer.insert_char(c),
            EditAction::DeleteCharBefore => buffer.delete_char_before(),
            EditAction::DeleteCharAfter => buffer.delete_char_after(),
            EditAction::CursorLeft => buffer.cursor_left(),
            EditAction::CursorRight => buffer.cursor_right(),
            EditAction::CursorWordLeft => buffer.cursor_word_left(),
            EditAction::CursorWordRight => buffer.cursor_word_right(),
            EditAction::SelectLeft => buffer.select_left(),
            EditAction::SelectRight => buffer.select_right(),
            EditAction::SelectWordLeft => buffer.select_word_left(),
            EditAction::SelectWordRight => buffer.select_word_right(),
            EditAction::CursorHome => buffer.cursor_home(),
            EditAction::CursorEnd => buffer.cursor_end(),
            EditAction::SelectHome => buffer.select_home(),
            EditAction::SelectEnd => buffer.select_end(),
            EditAction::DeleteWordBefore => buffer.delete_word_before(),
            EditAction::DeleteToStart => buffer.delete_to_start(),
            EditAction::DeleteToEnd => buffer.delete_to_end(),
            EditAction::SelectAll => buffer.select_all(),
            EditAction::Undo => buffer.undo(),
            EditAction::Redo => buffer.redo(),
            EditAction::NewLine => buffer.insert_newline(),
        }

        Ok(())
    }
}

/// Détermine le buffer actif selon le contexte de l'application.
fn get_active_buffer(state: &mut AppState) -> Option<TextBuffer<'_>> {
    // Priorité 1 : Vue Branches en mode Input
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.focus == BranchesFocus::Input
    {
        return Some(TextBuffer::new(
            &mut state.branches_view_state.input_text,
            &mut state.branches_view_state.input_cursor,
            &mut state.branches_view_state.input_selection_anchor,
            &mut state.branches_view_state.input_edit_history,
        ));
    }

    // Priorité 2 : Vue Staging en mode commit
    if state.staging_state.is_committing {
        return Some(TextBuffer::new(
            &mut state.staging_state.commit_message,
            &mut state.staging_state.cursor_position,
            &mut state.staging_state.selection_anchor,
            &mut state.staging_state.edit_history,
        ));
    }

    None
}

/// Applique une action d'edition a un champ texte arbitraire.
pub(super) fn edit_text(
    text: &mut String,
    cursor: &mut usize,
    selection_anchor: &mut Option<usize>,
    history: &mut TextEditHistory,
    action: EditAction,
) {
    let mut buffer = TextBuffer::new(text, cursor, selection_anchor, history);
    match action {
        EditAction::InsertChar(c) => buffer.insert_char(c),
        EditAction::DeleteCharBefore => buffer.delete_char_before(),
        EditAction::DeleteCharAfter => buffer.delete_char_after(),
        EditAction::CursorLeft => buffer.cursor_left(),
        EditAction::CursorRight => buffer.cursor_right(),
        EditAction::CursorWordLeft => buffer.cursor_word_left(),
        EditAction::CursorWordRight => buffer.cursor_word_right(),
        EditAction::SelectLeft => buffer.select_left(),
        EditAction::SelectRight => buffer.select_right(),
        EditAction::SelectWordLeft => buffer.select_word_left(),
        EditAction::SelectWordRight => buffer.select_word_right(),
        EditAction::CursorHome => buffer.cursor_home(),
        EditAction::CursorEnd => buffer.cursor_end(),
        EditAction::SelectHome => buffer.select_home(),
        EditAction::SelectEnd => buffer.select_end(),
        EditAction::DeleteWordBefore => buffer.delete_word_before(),
        EditAction::DeleteToStart => buffer.delete_to_start(),
        EditAction::DeleteToEnd => buffer.delete_to_end(),
        EditAction::SelectAll => buffer.select_all(),
        EditAction::Undo => buffer.undo(),
        EditAction::Redo => buffer.redo(),
        EditAction::NewLine => buffer.insert_newline(),
    }
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
        let mut selection_anchor = None;
        let mut history = TextEditHistory::default();
        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.insert_char('a');
        }
        assert_eq!(text, "a");
        assert_eq!(cursor, 1);

        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.insert_char('b');
        }
        assert_eq!(text, "ab");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_text_buffer_delete_char_before() {
        let mut text = "abc".to_string();
        let mut cursor = 2;
        let mut selection_anchor = None;
        let mut history = TextEditHistory::default();
        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.delete_char_before();
        }
        assert_eq!(text, "ac");
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_text_buffer_cursor_movement() {
        let mut text = "hello".to_string();
        let mut cursor = 2;
        let mut selection_anchor = None;
        let mut history = TextEditHistory::default();

        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.cursor_home();
        }
        assert_eq!(cursor, 0);

        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.cursor_end();
        }
        assert_eq!(cursor, 5);

        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
            buffer.cursor_left();
        }
        assert_eq!(cursor, 4);

        {
            let mut buffer =
                TextBuffer::new(&mut text, &mut cursor, &mut selection_anchor, &mut history);
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
    fn test_edit_handler_ignores_action_without_active_buffer() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = EditHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, EditAction::InsertChar('x'))
            .unwrap();

        assert!(ctx.state.staging_state.commit_message.is_empty());
        assert!(ctx.state.branches_view_state.input_text.is_empty());
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

    #[test]
    fn test_unicode_editing_keeps_cursor_on_character_boundaries() {
        let mut text = String::new();
        let mut cursor = 0;
        let mut selection_anchor = None;
        let mut history = TextEditHistory::default();

        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::InsertChar('é'),
        );
        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::InsertChar('t'),
        );
        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::CursorLeft,
        );
        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::DeleteCharBefore,
        );

        assert_eq!(text, "t");
        assert_eq!(cursor, 0);

        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::Undo,
        );
        assert_eq!(text, "ét");
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_word_selection_replacement_and_undo() {
        let mut text = "bonjour monde".to_string();
        let mut cursor = text.chars().count();
        let mut selection_anchor = None;
        let mut history = TextEditHistory::default();

        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::SelectWordLeft,
        );
        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::InsertChar('é'),
        );
        assert_eq!(text, "bonjour é");

        edit_text(
            &mut text,
            &mut cursor,
            &mut selection_anchor,
            &mut history,
            EditAction::Undo,
        );
        assert_eq!(text, "bonjour monde");
        assert_eq!(selection_range(cursor, selection_anchor), Some(8..13));
    }
}
