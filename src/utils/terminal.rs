//! Affichage des données Git non fiables sans commandes ni caractères de contrôle invisibles.

use std::borrow::Cow;

/// Échappe uniquement l'affichage ; les chemins et messages conservés par Git restent intacts.
pub fn terminal_text(text: &str) -> Cow<'_, str> {
    if !text.chars().any(needs_escape) {
        return Cow::Borrowed(text);
    }
    let mut rendered = String::with_capacity(text.len());
    for character in text.chars() {
        push_escaped(&mut rendered, character);
    }
    Cow::Owned(rendered)
}

fn push_escaped(rendered: &mut String, character: char) {
    match character {
        '\n' => rendered.push_str("\\n"),
        '\r' => rendered.push_str("\\r"),
        '\t' => rendered.push_str("\\t"),
        '\u{1b}' => rendered.push_str("\\x1b"),
        other if needs_escape(other) => rendered.extend(other.escape_unicode()),
        other => rendered.push(other),
    }
}

fn needs_escape(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Distingue les contrôles des séquences littérales dans les noms pouvant être ciblés par une action.
pub fn terminal_path(path: &str) -> Cow<'_, str> {
    if !path
        .chars()
        .any(|character| needs_escape(character) || matches!(character, '\\' | '"'))
    {
        return Cow::Borrowed(path);
    }
    let mut quoted = String::from("\"");
    for character in path.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            other => push_escaped(&mut quoted, other),
        }
    }
    quoted.push('"');
    Cow::Owned(quoted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_unicode_is_kept_without_an_allocation() {
        let source = "Résumé 漢字 e\u{301} 👩\u{200d}💻";
        assert!(matches!(terminal_text(source), Cow::Borrowed(value) if value == source));
    }

    #[test]
    fn controls_and_direction_overrides_are_explicit_and_inert() {
        let display = terminal_text("a\n\r\t\u{1b}]52;c;abc\u{7}\u{202e}b\u{2069}");
        assert!(!display.chars().any(char::is_control));
        assert_eq!(display, "a\\n\\r\\t\\x1b]52;c;abc\\u{7}\\u{202e}b\\u{2069}");
        assert_ne!(terminal_path("a\\nb"), terminal_path("a\nb"));
        assert_eq!(terminal_text("r\"a\\nb\""), "r\"a\\nb\"");
    }
}
