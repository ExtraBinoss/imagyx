use std::collections::{HashMap, HashSet};

use crate::{
    AppError,
    fuzzy::{exact_name_bonus, fts_query, normalize_query, reciprocal_rank},
    models::ImageAsset,
    state::AppState,
    tracing,
};

const MAX_SEARCH_RESULTS: usize = 60;
const MIN_LEXICAL_CANDIDATES: usize = 200;
const MAX_LEXICAL_CANDIDATES: usize = 500;
const SEMANTIC_CANDIDATES: usize = 200;

pub fn search(
    state: &AppState,
    query: &str,
    query_vector: Option<&[f32]>,
    folder_id: Option<&str>,
    requested_limit: usize,
    requested_offset: usize,
) -> Result<Vec<ImageAsset>, AppError> {
    let tokens = normalize_query(query);
    if tokens.is_empty() {
        let _trace = tracing::span("search.browse");
        return state
            .database
            .recent_images(folder_id, requested_limit, requested_offset);
    }

    let _trace = if query_vector.is_some() {
        tracing::span("search.hybrid")
    } else {
        tracing::span("search.lexical")
    };
    let limit = requested_limit.clamp(1, MAX_SEARCH_RESULTS);
    let lexical_limit = (limit * 4)
        .max(MIN_LEXICAL_CANDIDATES)
        .min(MAX_LEXICAL_CANDIDATES);
    let lexical = match fts_query(query) {
        Some(query) => state.database.lexical_search(&query, folder_id, lexical_limit)?,
        None => Vec::new(),
    };
    let semantic = query_vector.map_or_else(Vec::new, |query_vector| {
        state
            .vectors
            .read()
            .top_k(query_vector, folder_id, SEMANTIC_CANDIDATES)
    });

    let lexical_ranks = lexical
        .iter()
        .enumerate()
        .map(|(rank, asset)| (asset.id.clone(), rank))
        .collect::<HashMap<_, _>>();
    let semantic_ranks = semantic
        .iter()
        .enumerate()
        .map(|(rank, item)| (item.image_id.clone(), (rank, item.score)))
        .collect::<HashMap<_, _>>();

    let lexical_ids = lexical
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<HashSet<_>>();
    let missing_ids = semantic
        .iter()
        .filter(|item| !lexical_ids.contains(item.image_id.as_str()))
        .map(|item| item.image_id.clone())
        .collect::<Vec<_>>();
    let mut candidates = lexical;
    candidates.extend(state.database.images_by_ids(&missing_ids)?);

    let mut ranked = candidates
        .into_iter()
        .map(|mut asset| {
            let lexical_rank = lexical_ranks.get(&asset.id).copied();
            let semantic = semantic_ranks.get(&asset.id).copied();
            let score = reciprocal_rank(lexical_rank)
                + reciprocal_rank(semantic.map(|(rank, _)| rank))
                + exact_name_bonus(&asset, &tokens);
            asset.semantic_score = semantic.map(|(_, semantic_score)| semantic_score);
            (asset, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_asset, left_score), (right_asset, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| right_asset.modified_at.cmp(&left_asset.modified_at))
    });
    ranked.truncate(limit);
    Ok(ranked.into_iter().map(|(asset, _)| asset).collect())
}
