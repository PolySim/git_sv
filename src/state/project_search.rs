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

    let distance = levenshtein(&query, name);
    let allowed_distance = if query.chars().count() >= 8 { 2 } else { 1 };
    (distance <= allowed_distance).then_some(5_000 - distance as i64 * 500 - name.len() as i64)
}

fn subsequence_score(query: &str, candidate: &str) -> Option<i64> {
    let mut query_chars = query.chars();
    let mut wanted = query_chars.next()?;
    let mut previous_match = None;
    let mut score = 0i64;

    for (index, character) in candidate.chars().enumerate() {
        if character != wanted {
            continue;
        }

        score += if previous_match.is_some_and(|previous| previous + 1 == index) {
            80
        } else {
            20
        };
        if index == 0 || candidate.as_bytes().get(index.wrapping_sub(1)) == Some(&b'/') {
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

fn levenshtein(left: &str, right: &str) -> usize {
    let right: Vec<_> = right.chars().collect();
    let mut previous: Vec<_> = (0..=right.len()).collect();

    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_char) in right.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current.push(substitution.min(insertion).min(deletion));
        }
        previous = current;
    }

    previous[right.len()]
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
}
