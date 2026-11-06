//! Classement fuzzy des chemins de l'arborescence.

/// Calcule un score de pertinence pour un chemin.
///
/// Les correspondances exactes, préfixes et contiguës sont prioritaires. Une
/// petite distance d'édition sur le nom de fichier tolère les fautes courantes.
pub fn fuzzy_path_score(query: &str, path: &str) -> Option<i64> {
    let query = query.trim().to_lowercase();
    let path = path.to_lowercase();
    if query.is_empty() {
        return Some(0);
    }

    let name = path.rsplit('/').next().unwrap_or(&path);
    if path == query {
        return Some(20_000);
    }
    if name == query {
        return Some(18_000);
    }
    if name.starts_with(&query) {
        return Some(16_000 - name.len() as i64);
    }
    if let Some(index) = path.find(&query) {
        return Some(14_000 - index as i64 - path.len() as i64);
    }
    if let Some(score) = subsequence_score(&query, &path) {
        return Some(10_000 + score);
    }

    let allowed_distance = if query.chars().count() >= 8 { 2 } else { 1 };
    let distance = bounded_levenshtein(&query, name, allowed_distance)?;
    Some(5_000 - distance as i64 * 500 - name.len() as i64)
}

fn subsequence_score(query: &str, candidate: &str) -> Option<i64> {
    let mut query_chars = query.chars();
    let mut wanted = query_chars.next()?;
    let mut previous_match = None;
    let mut previous_character = None;
    let mut score = 0i64;

    for (index, character) in candidate.chars().enumerate() {
        let starts_component = previous_character.is_none() || previous_character == Some('/');
        previous_character = Some(character);
        if character != wanted {
            continue;
        }

        score += if previous_match.is_some_and(|previous| previous + 1 == index) {
            80
        } else {
            20
        };
        if starts_component {
            score += 100;
        }
        previous_match = Some(index);

        let Some(next) = query_chars.next() else {
            return Some(score - candidate.chars().count() as i64);
        };
        wanted = next;
    }

    None
}

fn bounded_levenshtein(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    let right: Vec<_> = right.chars().collect();
    if left.chars().count().abs_diff(right.len()) > max_distance {
        return None;
    }
    let out_of_range = max_distance + 1;
    let mut previous: Vec<_> = (0..=right.len())
        .map(|index| index.min(out_of_range))
        .collect();
    let mut current = vec![out_of_range; right.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        let row = left_index + 1;
        current.fill(out_of_range);
        current[0] = row.min(out_of_range);
        let start = row.saturating_sub(max_distance).max(1);
        let end = (row + max_distance).min(right.len());
        let mut row_minimum = current[0];
        // Seule la bande proche de la diagonale peut respecter la tolérance.
        for column in start..=end {
            let substitution = previous[column - 1] + usize::from(left_char != right[column - 1]);
            let insertion = current[column - 1] + 1;
            let deletion = previous[column] + 1;
            current[column] = substitution.min(insertion).min(deletion);
            row_minimum = row_minimum.min(current[column]);
        }
        if row_minimum > max_distance {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }

    (previous[right.len()] <= max_distance).then_some(previous[right.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_prefix_matches_rank_before_subsequences() {
        let exact = fuzzy_path_score("main.rs", "src/main.rs").unwrap();
        let prefix = fuzzy_path_score("mai", "src/main.rs").unwrap();
        let subsequence = fuzzy_path_score("mr", "src/main.rs").unwrap();

        assert!(exact > prefix);
        assert!(prefix > subsequence);
    }

    #[test]
    fn filename_search_tolerates_a_typo() {
        assert!(fuzzy_path_score("maim.rs", "src/main.rs").is_some());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        assert_eq!(fuzzy_path_score("widget", "src/main.rs"), None);
    }

    #[test]
    fn audit_unicode_directory_boundaries_score_like_ascii() {
        assert_eq!(
            subsequence_score("éa", "é/a"),
            subsequence_score("ea", "e/a")
        );
    }

    #[test]
    fn audit_bounded_edit_distance_preserves_typo_tolerance() {
        let examples = [
            ("", "", 0, Some(0)),
            ("a", "", 1, Some(1)),
            ("", "a", 1, Some(1)),
            ("maim.rs", "main.rs", 1, Some(1)),
            ("min.rs", "main.rs", 1, Some(1)),
            ("mainn.rs", "main.rs", 1, Some(1)),
            ("mian.rs", "main.rs", 1, None),
            ("mian.rs", "main.rs", 2, Some(2)),
            ("élément", "element", 2, Some(2)),
            ("abcdef", "uvwxyz", 2, None),
            ("long unrelated query", "a", 2, None),
        ];
        for (left, right, limit, expected) in examples {
            assert_eq!(
                bounded_levenshtein(left, right, limit),
                expected,
                "{left} / {right}"
            );
        }
    }
}
