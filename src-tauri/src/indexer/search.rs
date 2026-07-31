use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    AppError,
    fuzzy::{
        dominance_requested, expand_search_tokens, fts_query, fts_query_or, meaningful_tokens,
        name_match_quality, normalize_query, parse_colors, reciprocal_rank, token_coverage,
    },
    models::SearchPage,
    state::AppState,
    tracing,
};

const MAX_QUERY_PAGE_SIZE: usize = 200;
const MAX_SEARCH_WINDOW: usize = 50_000;
const MIN_SEARCH_CANDIDATES: usize = 200;
const CANDIDATE_OVERSAMPLE: usize = 4;
const COLOR_CANDIDATE_LIMIT: usize = 800;

#[allow(clippy::too_many_arguments)]
pub fn search_page_with_diagnostics(
    state: &AppState,
    query: &str,
    query_vector: Option<&[f32]>,
    folder_id: Option<&str>,
    requested_mode: Option<&str>,
    exclude_image_id: Option<&str>,
    requested_colors: Option<&[String]>,
    dominant_color: bool,
    requested_limit: usize,
    requested_offset: usize,
    diagnostic_id: Option<&str>,
) -> Result<SearchPage, AppError> {
    let started = Instant::now();
    let visual_mode =
        requested_mode == Some("visual") || (query.trim().is_empty() && query_vector.is_some());
    if visual_mode {
        return visual_search(
            state,
            query_vector,
            folder_id,
            exclude_image_id,
            requested_limit,
            requested_offset,
            diagnostic_id,
            started,
        );
    }

    let tokens = normalize_query(query);
    let meaningful = meaningful_tokens(&tokens);
    if query.trim().is_empty() {
        let limit = requested_limit.clamp(1, MAX_SEARCH_WINDOW);
        let offset = requested_offset.min(MAX_SEARCH_WINDOW);
        let total = state.database.image_count(folder_id)?;
        let items = state.database.recent_images(folder_id, limit, offset)?;
        trace_search(diagnostic_id, "browse", query, items.len(), total, started);
        return Ok(SearchPage { items, total });
    }
    // A non-empty query with no FTS tokens is punctuation, not a request to
    // browse the library.
    if tokens.is_empty() {
        trace_search(diagnostic_id, "empty", query, 0, 0, started);
        return Ok(SearchPage {
            items: Vec::new(),
            total: 0,
        });
    }

    let limit = requested_limit.clamp(1, MAX_QUERY_PAGE_SIZE);
    let offset = requested_offset.min(MAX_SEARCH_WINDOW);
    let window_end = offset.saturating_add(limit).min(MAX_SEARCH_WINDOW);
    let candidate_limit = window_end
        .saturating_mul(CANDIDATE_OVERSAMPLE)
        .max(MIN_SEARCH_CANDIDATES)
        .min(MAX_SEARCH_WINDOW);
    let colors = requested_colors
        .map(|values| parse_colors(&values.join(" ")))
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| parse_colors(query));
    let subject_tokens = meaningful
        .iter()
        .filter(|token| parse_colors(token).is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let ranking_tokens = if subject_tokens.is_empty() {
        &meaningful
    } else {
        &subject_tokens
    };
    let dominant = dominant_color || dominance_requested(query);
    let color_limit = if colors.is_empty() {
        0
    } else {
        COLOR_CANDIDATE_LIMIT.max(candidate_limit)
    };

    let strict_query = fts_query(query);
    let mut prepared_query = strict_query;
    let mut lexical = prepared_query
        .as_deref()
        .map(|fts| {
            state
                .database
                .lexical_search(fts, folder_id, candidate_limit)
        })
        .transpose()?
        .unwrap_or_default();
    // FTS AND is the precise path. If it is too strict, OR keeps a useful
    // lexical result visible while the semantic/color ranker fills the gaps.
    if lexical.is_empty() && tokens.len() > 1 {
        prepared_query = fts_query_or(query);
        lexical = prepared_query
            .as_deref()
            .map(|fts| {
                state
                    .database
                    .lexical_search(fts, folder_id, candidate_limit)
            })
            .transpose()?
            .unwrap_or_default();
    }
    if lexical.is_empty() && query_vector.is_none() {
        // Keep the normal FTS path immediate, but make a first typo query
        // able to use the lazily loaded fuzzy index as well.
        state.load_vectors()?;
    }

    let lexical_total = prepared_query
        .as_deref()
        .map(|fts| state.database.lexical_search_count(fts, folder_id))
        .transpose()?
        .unwrap_or(0);

    let semantic = query_vector.map_or_else(Vec::new, |vector| {
        state.vectors.read().top_k_with_diagnostics(
            vector,
            folder_id,
            candidate_limit,
            diagnostic_id,
        )
    });
    let fuzzy_tokens = expand_search_tokens(&meaningful);
    let fuzzy = state
        .fuzzy
        .read()
        .search(&fuzzy_tokens, folder_id, candidate_limit);
    let color_matches = if colors.is_empty() {
        Vec::new()
    } else {
        state
            .colors
            .read()
            .top_k(&colors, dominant, folder_id, color_limit)
    };

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
    let fuzzy_scores = fuzzy
        .into_iter()
        .map(|item| (item.image_id, item.score))
        .collect::<HashMap<_, _>>();
    let color_scores = color_matches
        .into_iter()
        .map(|item| (item.image_id, item.score))
        .collect::<HashMap<_, _>>();

    let mut candidate_ids = lexical
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<HashSet<_>>();
    candidate_ids.extend(semantic.iter().map(|item| item.image_id.clone()));
    candidate_ids.extend(fuzzy_scores.keys().cloned());
    candidate_ids.extend(color_scores.keys().cloned());
    let missing_ids = candidate_ids
        .iter()
        .filter(|id| !lexical_ranks.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    let mut candidates = lexical;
    candidates.extend(state.database.images_by_ids(&missing_ids)?);
    let mut seen = HashSet::new();
    candidates.retain(|asset| seen.insert(asset.id.clone()));

    let mut ranked = candidates
        .into_iter()
        .map(|mut asset| {
            let lexical_rank = lexical_ranks.get(&asset.id).copied();
            let semantic_match = semantic_ranks.get(&asset.id).copied();
            let semantic_score = semantic_match.map(|(_, score)| score).unwrap_or(0.0);
            let color_score = color_scores.get(&asset.id).copied().unwrap_or(0.0);
            let fuzzy_score = fuzzy_scores.get(&asset.id).copied().unwrap_or(0.0);
            let name_score = name_match_quality(&asset, ranking_tokens);
            let coverage = token_coverage(&asset, ranking_tokens);
            let lexical_rank_score = reciprocal_rank(lexical_rank)
                / reciprocal_rank(Some(0)).max(f32::EPSILON);
            let semantic_rank_score = reciprocal_rank(semantic_match.map(|(rank, _)| rank))
                / reciprocal_rank(Some(0)).max(f32::EPSILON);
            let score = fused_relevance_score(
                !subject_tokens.is_empty(),
                !colors.is_empty(),
                name_score,
                coverage,
                semantic_score,
                color_score,
                fuzzy_score,
                lexical_rank_score,
                semantic_rank_score,
            );
            asset.semantic_score = semantic_match.map(|(_, score)| score);
            asset.relevance_score = Some(score.clamp(0.0, 1.0));
            (asset, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_asset, left_score), (right_asset, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| right_asset.modified_at.cmp(&left_asset.modified_at))
            .then_with(|| left_asset.id.cmp(&right_asset.id))
    });

    let vector_count = query_vector.map_or(0, |_| state.vectors.read().count(folder_id));
    let lexical_only = if query_vector.is_some() {
        prepared_query
            .as_deref()
            .map(|fts| {
                state
                    .database
                    .lexical_search_count_without_embeddings(fts, folder_id)
            })
            .transpose()?
            .unwrap_or(0)
    } else {
        0
    };
    let total = if query_vector.is_some() {
        vector_count.saturating_add(lexical_only).max(ranked.len())
    } else {
        lexical_total.max(ranked.len())
    };
    let items = ranked
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(asset, _)| asset)
        .collect::<Vec<_>>();
    let mode = if query_vector.is_some() {
        "hybrid"
    } else {
        "lexical"
    };
    trace_search(diagnostic_id, mode, query, items.len(), total, started);
    Ok(SearchPage { items, total })
}

fn fused_relevance_score(
    has_subject: bool,
    has_color: bool,
    name_score: f32,
    coverage: f32,
    semantic_score: f32,
    color_score: f32,
    fuzzy_score: f32,
    lexical_rank_score: f32,
    semantic_rank_score: f32,
) -> f32 {
    let score = match (has_subject, has_color) {
        // A requested subject must decide the result. Colour is a useful
        // tie-breaker but cannot promote a green logo over a filename/subject
        // match such as girl_train_segmented.png.
        (true, true) => 0.34 * name_score
            + 0.14 * coverage
            + 0.42 * semantic_score
            + 0.07 * color_score
            + 0.02 * fuzzy_score
            + 0.005 * lexical_rank_score
            + 0.005 * semantic_rank_score,
        (true, false) => 0.36 * name_score
            + 0.16 * coverage
            + 0.44 * semantic_score
            + 0.02 * fuzzy_score
            + 0.01 * lexical_rank_score
            + 0.01 * semantic_rank_score,
        // Pure colour requests should be driven by the compact colour
        // signature, with the text embedding as a secondary signal.
        (false, true) => 0.04 * name_score
            + 0.04 * coverage
            + 0.20 * semantic_score
            + 0.70 * color_score
            + 0.01 * lexical_rank_score
            + 0.01 * semantic_rank_score,
        (false, false) => 0.20 * semantic_score
            + 0.70 * color_score
            + 0.05 * fuzzy_score
            + 0.03 * lexical_rank_score
            + 0.02 * semantic_rank_score,
    };
    score.clamp(0.0, 1.0)
}

#[allow(clippy::too_many_arguments)]
fn visual_search(
    state: &AppState,
    query_vector: Option<&[f32]>,
    folder_id: Option<&str>,
    exclude_image_id: Option<&str>,
    requested_limit: usize,
    requested_offset: usize,
    diagnostic_id: Option<&str>,
    started: Instant,
) -> Result<SearchPage, AppError> {
    let Some(query_vector) = query_vector else {
        return Ok(SearchPage {
            items: Vec::new(),
            total: 0,
        });
    };
    let limit = requested_limit.clamp(1, MAX_QUERY_PAGE_SIZE);
    let offset = requested_offset.min(MAX_SEARCH_WINDOW);
    let vectors = state.vectors.read();
    let excluded_count =
        exclude_image_id.is_some_and(|id| vectors.contains_in_scope(id, folder_id)) as usize;
    let total = vectors.count(folder_id).saturating_sub(excluded_count);
    let matches = vectors
        .top_k_with_diagnostics(
            query_vector,
            folder_id,
            offset.saturating_add(limit).saturating_add(excluded_count),
            diagnostic_id,
        )
        .into_iter()
        .filter(|item| exclude_image_id != Some(item.image_id.as_str()))
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    drop(vectors);
    let ids = matches
        .iter()
        .map(|item| item.image_id.clone())
        .collect::<Vec<_>>();
    let assets = state
        .database
        .images_by_ids(&ids)?
        .into_iter()
        .map(|asset| (asset.id.clone(), asset))
        .collect::<HashMap<_, _>>();
    let items = matches
        .into_iter()
        .filter_map(|item| {
            let mut asset = assets.get(&item.image_id)?.clone();
            asset.semantic_score = Some(item.score);
            asset.relevance_score = Some(item.score);
            Some(asset)
        })
        .collect::<Vec<_>>();
    trace_search(diagnostic_id, "visual", "", items.len(), total, started);
    Ok(SearchPage { items, total })
}

fn trace_search(
    diagnostic_id: Option<&str>,
    mode: &str,
    query: &str,
    results: usize,
    total: usize,
    started: Instant,
) {
    #[cfg(debug_assertions)]
    tracing::event(
        "search.pipeline",
        format!(
            "id={} mode={mode} query={query:?} results={results} total_available={total} total_ms={:.2}",
            diagnostic_id.unwrap_or("-"),
            started.elapsed().as_secs_f64() * 1_000.0,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::fused_relevance_score;

    #[test]
    fn subject_match_beats_a_green_brand_asset_for_subject_colour_query() {
        let girl = fused_relevance_score(true, true, 0.82, 1.0, 0.06, 0.82, 1.0, 0.0, 0.0);
        let brand = fused_relevance_score(true, true, 0.0, 0.0, 0.58, 1.0, 0.0, 0.0, 0.0);

        assert!(girl > brand, "girl={girl:.3} brand={brand:.3}");
    }
}
