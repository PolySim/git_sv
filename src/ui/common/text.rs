//! Utilitaires de manipulation de texte Unicode-safe.

#![allow(dead_code)]

use ratatui::{style::Style, text::Span};

/// Tronque une chaîne de manière safe pour Unicode.
///
/// # Paramètres
/// * `s` - Chaîne à tronquer
/// * `max_len` - Largeur maximale en cellules du terminal
/// * `ellipsis` - Ajouter "…" si tronqué
pub fn truncate(s: &str, max_len: usize, ellipsis: bool) -> String {
    let span = Span::raw(s);
    if span.width() <= max_len {
        return s.to_string();
    }
    let suffix = if ellipsis && max_len > 0 { "…" } else { "" };
    let budget = max_len.saturating_sub(Span::raw(suffix).width());
    let mut result = String::new();
    let mut used = 0;
    for grapheme in span.styled_graphemes(Style::default()) {
        let width = Span::raw(grapheme.symbol).width();
        if used + width > budget {
            break;
        }
        result.push_str(grapheme.symbol);
        used += width;
    }
    result.push_str(suffix);
    result
}

/// Tronque une chaîne au début (garde la fin).
pub fn truncate_start(s: &str, max_len: usize, ellipsis: bool) -> String {
    let span = Span::raw(s);
    if span.width() <= max_len {
        return s.to_string();
    }
    let prefix = if ellipsis && max_len > 0 { "…" } else { "" };
    let budget = max_len.saturating_sub(Span::raw(prefix).width());
    let graphemes: Vec<_> = span.styled_graphemes(Style::default()).collect();
    let mut used = 0;
    let mut start = graphemes.len();
    for grapheme in graphemes.iter().rev() {
        let width = Span::raw(grapheme.symbol).width();
        if used + width > budget {
            break;
        }
        used += width;
        start -= 1;
    }
    let mut result = prefix.to_string();
    for grapheme in &graphemes[start..] {
        result.push_str(grapheme.symbol);
    }
    result
}

/// Pad une chaîne à droite jusqu'à la longueur spécifiée.
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = Span::raw(s).width();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

/// Pad une chaîne à gauche jusqu'à la longueur spécifiée.
pub fn pad_left(s: &str, width: usize) -> String {
    let char_count = Span::raw(s).width();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - char_count), s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10, true), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5, true), "hell…");
        assert_eq!(truncate("hello world", 5, false), "hello");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("héllo wörld", 5, true), "héll…");
    }

    #[test]
    fn test_truncate_start() {
        assert_eq!(
            truncate_start("/a/very/long/path/file.rs", 15, true),
            "…g/path/file.rs"
        );
    }

    #[test]
    fn terminal_width_and_grapheme_boundaries_are_preserved() {
        assert_eq!(truncate("👩\u{200d}💻漢字", 3, true), "👩\u{200d}💻…");
        assert_eq!(truncate("e\u{301}🙂漢", 3, true), "e\u{301}…");
        assert_eq!(truncate_start("漢👩\u{200d}💻", 3, true), "…👩\u{200d}💻");
        assert_eq!(truncate("漢", 1, false), "");
        assert_eq!(truncate("漢", 0, true), "");
        assert_eq!(truncate_start("漢", 0, true), "");
        assert_eq!(pad_right("漢", 3), "漢 ");
        assert_eq!(pad_left("e\u{301}", 3), "  e\u{301}");
    }
}
