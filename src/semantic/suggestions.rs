// src/semantic/suggestions.rs
use crate::ast::ModelDecl;

/// Computes the Levenshtein distance between two strings.
/// This algorithm calculates the minimum number of single-character edits (insertions, deletions or substitutions)
/// required to change one word into the other.
pub(crate) fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut dp = vec![vec![0; len_b + 1]; len_a + 1];

    for (i, row) in dp.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(len_b + 1) {
        *cell = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    dp[len_a][len_b]
}

/// Suggests a similar field name in a database model declaration if there is a close match.
pub(crate) fn suggest_similar_field(name: &str, model: &ModelDecl) -> Option<String> {
    let mut best_candidate = None;
    let mut min_distance = 3;
    for field in &model.fields {
        let dist = levenshtein_distance(name, &field.name);
        if dist < min_distance {
            min_distance = dist;
            best_candidate = Some(field.name.clone());
        }
    }
    best_candidate
}

pub(crate) fn suggest_from_list(input: &str, options: &[&str]) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for &opt in options {
        let d = levenshtein_distance(input, opt);
        if d <= 2 {
            match best {
                None => best = Some((opt, d)),
                Some((_, prev_d)) if d < prev_d => best = Some((opt, d)),
                _ => {}
            }
        }
    }
    best.map(|(s, _)| s.to_string())
}
