use crate::models::ImageAsset;

const RRF_K: f32 = 60.0;

pub fn reciprocal_rank(rank: Option<usize>) -> f32 {
    rank.map_or(0.0, |rank| 1.0 / (RRF_K + rank as f32 + 1.0))
}

pub fn exact_name_bonus(asset: &ImageAsset, tokens: &[String]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let name = asset.name.to_lowercase();
    let matched = tokens.iter().filter(|token| name.contains(token.as_str())).count();
    0.004 * matched as f32 / tokens.len() as f32
}

#[cfg(test)]
mod tests {
    use super::{exact_name_bonus, reciprocal_rank};
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
            semantic_score: None,
        };
        assert!(exact_name_bonus(&asset, &["green".into(), "woman".into()]) > 0.0);
    }
}
