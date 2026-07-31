pub fn normalize_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in query.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            current.extend(strip_diacritic(character).chars());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn strip_diacritic(character: char) -> String {
    match character {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a".into(),
        'ç' => "c".into(),
        'è' | 'é' | 'ê' | 'ë' => "e".into(),
        'ì' | 'í' | 'î' | 'ï' => "i".into(),
        'ñ' => "n".into(),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => "o".into(),
        'ù' | 'ú' | 'û' | 'ü' => "u".into(),
        'ý' | 'ÿ' => "y".into(),
        'æ' => "ae".into(),
        'œ' => "oe".into(),
        _ => character.to_string(),
    }
}

pub fn fts_query(query: &str) -> Option<String> {
    build_fts_query(query, " AND ")
}

pub fn fts_query_or(query: &str) -> Option<String> {
    build_fts_query(query, " OR ")
}

fn build_fts_query(query: &str, operator: &str) -> Option<String> {
    let tokens = normalize_query(query);
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(operator),
    )
}

#[cfg(test)]
mod tests {
    use super::{fts_query, fts_query_or, normalize_query};

    #[test]
    fn strips_punctuation_but_keeps_useful_separators() {
        assert_eq!(
            normalize_query("  Woman, green-blue_été!  "),
            vec!["woman", "green", "blue", "ete"]
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

    #[test]
    fn can_fallback_to_any_token() {
        assert_eq!(
            fts_query_or("woman green").as_deref(),
            Some("\"woman\"* OR \"green\"*")
        );
    }
}
