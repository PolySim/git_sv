//! Utilitaires de manipulation de texte Unicode-safe.

/// Tronque une chaîne de manière safe pour Unicode.
///
/// # Arguments
/// * `s` - Chaîne à tronquer
/// * `max_len` - Longueur maximale en caractères
/// * `ellipsis` - Ajouter "…" si tronqué
pub fn truncate(s: &str, max_len: usize, ellipsis: bool) -> String {
    let char_count = s.chars().count();

    if char_count <= max_len {
        s.to_string()
    } else if ellipsis && max_len > 1 {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    } else {
        s.chars().take(max_len).collect()
    }
}

/// Tronque une chaîne au début (garde la fin).
pub fn truncate_start(s: &str, max_len: usize, ellipsis: bool) -> String {
    let char_count = s.chars().count();

    if char_count <= max_len {
        s.to_string()
    } else if ellipsis && max_len > 1 {
        let keep = max_len - 1;
        let skip = char_count.saturating_sub(keep);
        let truncated: String = s.chars().skip(skip).collect();
        format!("…{}", truncated)
    } else {
        s.chars().skip(char_count - max_len).collect()
    }
}

/// Pad une chaîne à droite jusqu'à la longueur spécifiée.
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

/// Pad une chaîne à gauche jusqu'à la longueur spécifiée.
pub fn pad_left(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
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
}
