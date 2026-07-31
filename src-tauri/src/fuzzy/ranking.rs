use super::{expand_search_tokens, normalize_query};
use crate::models::ImageAsset;

const RRF_K: f32 = 60.0;

pub fn reciprocal_rank(rank: Option<usize>) -> f32 {
    rank.map_or(0.0, |rank| 1.0 / (RRF_K + rank as f32 + 1.0))
}

pub fn name_match_quality(asset: &ImageAsset, tokens: &[String]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let name_tokens = normalize_query(&asset.name);
    let matched = tokens
        .iter()
        .filter_map(|token| {
            if name_tokens.iter().any(|name| name == token) {
                Some(1.0)
            } else if expand_search_tokens(std::slice::from_ref(token))
                .iter()
                .skip(1)
                .any(|alias| name_tokens.iter().any(|name| name == alias))
            {
                Some(0.82)
            } else if name_tokens.iter().any(|name| name.starts_with(token)) {
                Some(0.75)
            } else {
                None
            }
        })
        .sum::<f32>();
    (matched / tokens.len() as f32).clamp(0.0, 1.0)
}

pub fn token_coverage(asset: &ImageAsset, tokens: &[String]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let name_tokens = normalize_query(&asset.name);
    let matched = tokens
        .iter()
        .filter(|token| {
            name_tokens
                .iter()
                .any(|name| {
                    name == *token
                        || name.starts_with(token.as_str())
                        || expand_search_tokens(std::slice::from_ref(token))
                            .iter()
                            .skip(1)
                            .any(|alias| name == alias || name.starts_with(alias))
                })
        })
        .count();
    matched as f32 / tokens.len() as f32
}

#[cfg(test)]
mod tests {
    use super::{name_match_quality, reciprocal_rank};
    use crate::models::ImageAsset;

    #[test]
    fn higher_ranks_are_more_important() {
        assert!(reciprocal_rank(Some(0)) > reciprocal_rank(Some(10)));
        assert_eq!(reciprocal_rank(None), 0.0);
    }

    #[test]
    fn filename_matches_receive_a_small_bonus() {
        let asset = ImageAsset {
            id: "1".into(),
            folder_id: "folder".into(),
            path: "/tmp/green-woman.jpg".into(),
            name: "green-woman.jpg".into(),
            extension: "jpg".into(),
            width: 1,
            height: 1,
            size_bytes: 1,
            modified_at: 1,
            thumbnail_path: String::new(),
            color_signature: None,
            semantic_score: None,
            relevance_score: None,
        };
        assert!(name_match_quality(&asset, &["green".into(), "woman".into()]) > 0.0);
        assert!(name_match_quality(&asset, &["art".into()]) == 0.0);
    }

    #[test]
    fn woman_queries_retrieve_girl_named_assets() {
        let asset = ImageAsset {
            id: "girl".into(),
            folder_id: "folder".into(),
            path: "/tmp/girl_train_segmented.png".into(),
            name: "girl_train_segmented.png".into(),
            extension: "png".into(),
            width: 1,
            height: 1,
            size_bytes: 1,
            modified_at: 1,
            thumbnail_path: String::new(),
            color_signature: None,
            semantic_score: None,
            relevance_score: None,
        };

        assert!(name_match_quality(&asset, &["woman".into()]) > 0.8);
        assert_eq!(token_coverage(&asset, &["woman".into()]), 1.0);
    }
}
