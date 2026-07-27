pub fn normalize_query(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

pub fn fts_query(query: &str) -> Option<String> {
    let tokens = normalize_query(query);
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

#[cfg(test)]
mod tests {
    use super::{fts_query, normalize_query};

    #[test]
    fn strips_punctuation_but_keeps_useful_separators() {
        assert_eq!(
            normalize_query("  Woman, green-blue!  "),
            vec!["woman", "green-blue"]
        );
    }

    #[test]
    fn builds_a_safe_prefix_query() {
        assert_eq!(
            fts_query("woman green").as_deref(),
            Some("\"woman\"* AND \"green\"*")
        );
    }

    #[test]
    fn empty_queries_do_not_hit_fts() {
        assert_eq!(fts_query(" .. "), None);
    }
}
