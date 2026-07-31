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

pub fn meaningful_tokens(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.as_str(),
                "a" | "an" | "and" | "avec" | "de" | "des" | "du" | "en" | "et"
                    | "la" | "le" | "les" | "of" | "par" | "pour" | "the" | "un"
                    | "une" | "with"
            )
        })
        .cloned()
        .collect()
}

pub fn expand_search_tokens(tokens: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    for token in meaningful_tokens(tokens) {
        let aliases: &[&str] = match token.as_str() {
            "woman" | "women" | "femme" | "femmes" | "female" => {
                &["woman", "women", "femme", "femmes", "girl", "girls", "fille", "filles", "female"]
            }
            "girl" | "girls" | "fille" | "filles" => {
                &["girl", "girls", "fille", "filles", "woman", "women", "femme", "femmes", "female"]
            }
            "man" | "men" | "homme" | "hommes" | "male" => {
                &["man", "men", "homme", "hommes", "boy", "boys", "garcon", "garcons", "male"]
            }
            "boy" | "boys" | "garcon" | "garcons" => {
                &["boy", "boys", "garcon", "garcons", "man", "men", "homme", "hommes", "male"]
            }
            "person" | "people" | "personne" | "personnes" | "human" | "humain" => {
                &["person", "people", "personne", "personnes", "human", "humain"]
            }
            _ => &[token.as_str()],
        };
        for alias in aliases {
            let alias = (*alias).to_owned();
            if !expanded.contains(&alias) {
                expanded.push(alias);
            }
        }
    }
    expanded
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
    let tokens = meaningful_tokens(&normalize_query(query));
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
    use super::{expand_search_tokens, fts_query, fts_query_or, meaningful_tokens, normalize_query};

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

    #[test]
    fn expands_human_subjects_without_expanding_connectors() {
        let tokens = normalize_query("woman and green");
        assert_eq!(meaningful_tokens(&tokens), vec!["woman", "green"]);
        assert!(expand_search_tokens(&tokens).contains(&"girl".to_owned()));
        assert!(!expand_search_tokens(&tokens).contains(&"and".to_owned()));
    }
}
