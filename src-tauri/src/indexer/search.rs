use std::collections::{HashMap, HashSet};

#[cfg(debug_assertions)]
use std::time::Instant;

use crate::{
    AppError,
    fuzzy::{exact_name_bonus, fts_query, normalize_query, reciprocal_rank},
    models::SearchPage,
    state::AppState,
    tracing,
};

const MAX_QUERY_PAGE_SIZE: usize = 200;
const MAX_SEARCH_WINDOW: usize = 50_000;
const MIN_SEARCH_CANDIDATES: usize = 200;
const CANDIDATE_OVERSAMPLE: usize = 4;

pub fn search_page_with_diagnostics(
    state: &AppState,
    query: &str,
    query_vector: Option<&[f32]>,
    folder_id: Option<&str>,
    requested_limit: usize,
    requested_offset: usize,
    _diagnostic_id: Option<&str>,
) -> Result<SearchPage, AppError> {
    #[cfg(debug_assertions)]
    let diagnostic_id = _diagnostic_id;
    #[cfg(debug_assertions)]
    let total_started_at = Instant::now();
    #[cfg(debug_assertions)]
    let normalize_started_at = Instant::now();

    let tokens = normalize_query(query);

    #[cfg(debug_assertions)]
    let normalize_ms = normalize_started_at.elapsed().as_secs_f64() * 1_000.0;

    if tokens.is_empty() {
        let _trace = tracing::span("search.browse");
        let limit = requested_limit.max(1).min(MAX_SEARCH_WINDOW);
        let offset = requested_offset.min(MAX_SEARCH_WINDOW);
        #[cfg(debug_assertions)]
        let count_started_at = Instant::now();
        let total = state.database.image_count(folder_id)?;
        #[cfg(debug_assertions)]
        let count_ms = count_started_at.elapsed().as_secs_f64() * 1_000.0;
        #[cfg(debug_assertions)]
        let database_started_at = Instant::now();
        let items = state.database.recent_images(folder_id, limit, offset)?;

        #[cfg(debug_assertions)]
        tracing::event(
            "search.pipeline",
            format!(
                "id={} mode=browse query={query:?} folder_id={folder_id:?} requested_limit={requested_limit} effective_limit={limit} offset={offset} normalize_ms={normalize_ms:.2} count_ms={count_ms:.2} database_ms={:.2} results={} total_available={total} total_ms={:.2}",
                diagnostic_id.unwrap_or("-"),
                database_started_at.elapsed().as_secs_f64() * 1_000.0,
                items.len(),
                total_started_at.elapsed().as_secs_f64() * 1_000.0,
            ),
        );
        return Ok(SearchPage { items, total });
    }

    #[cfg(debug_assertions)]
    let mode = if query_vector.is_some() {
        "hybrid"
    } else {
        "lexical"
    };
    let _trace = if query_vector.is_some() {
        tracing::span("search.hybrid")
    } else {
        tracing::span("search.lexical")
    };

    let limit = requested_limit.clamp(1, MAX_QUERY_PAGE_SIZE);
    let offset = requested_offset.min(MAX_SEARCH_WINDOW);
    let window_end = offset.saturating_add(limit).min(MAX_SEARCH_WINDOW);
    let candidate_limit = window_end
        .saturating_mul(CANDIDATE_OVERSAMPLE)
        .max(MIN_SEARCH_CANDIDATES)
        .min(MAX_SEARCH_WINDOW);

    #[cfg(debug_assertions)]
    let fts_started_at = Instant::now();
    let prepared_fts_query = fts_query(query);
    #[cfg(debug_assertions)]
    let fts_prepare_ms = fts_started_at.elapsed().as_secs_f64() * 1_000.0;

    #[cfg(debug_assertions)]
    let count_started_at = Instant::now();
    let total = if query_vector.is_some() {
        state
            .database
            .hybrid_search_count(prepared_fts_query.as_deref(), folder_id)?
    } else {
        match prepared_fts_query.as_deref() {
            Some(prepared_query) => state.database.lexical_search_count(prepared_query, folder_id)?,
            None => 0,
        }
    };
    #[cfg(debug_assertions)]
    let count_ms = count_started_at.elapsed().as_secs_f64() * 1_000.0;

    #[cfg(debug_assertions)]
    let lexical_started_at = Instant::now();
    let lexical = match prepared_fts_query.as_deref() {
        Some(prepared_query) => state
            .database
            .lexical_search(prepared_query, folder_id, candidate_limit)?,
        None => Vec::new(),
    };
    #[cfg(debug_assertions)]
    let lexical_ms = lexical_started_at.elapsed().as_secs_f64() * 1_000.0;
    #[cfg(debug_assertions)]
    let lexical_count = lexical.len();

    #[cfg(debug_assertions)]
    let mut vector_lock_wait_ms = 0.0;
    #[cfg(debug_assertions)]
    let mut vector_scan_ms = 0.0;
    #[cfg(debug_assertions)]
    let mut vector_store_len = 0usize;

    let semantic = query_vector.map_or_else(Vec::new, |query_vector| {
        #[cfg(debug_assertions)]
        let vector_lock_started_at = Instant::now();
        let vectors = state.vectors.read();
        #[cfg(debug_assertions)]
        {
            vector_lock_wait_ms = vector_lock_started_at.elapsed().as_secs_f64() * 1_000.0;
            vector_store_len = vectors.len();
        }
        #[cfg(debug_assertions)]
        let vector_scan_started_at = Instant::now();
        let matches = vectors.top_k_with_diagnostics(
            query_vector,
            folder_id,
            candidate_limit,
            _diagnostic_id,
        );
        #[cfg(debug_assertions)]
        {
            vector_scan_ms = vector_scan_started_at.elapsed().as_secs_f64() * 1_000.0;
        }
        matches
    });
    #[cfg(debug_assertions)]
    let semantic_count = semantic.len();

    #[cfg(debug_assertions)]
    let rank_maps_started_at = Instant::now();
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
    #[cfg(debug_assertions)]
    let rank_maps_ms = rank_maps_started_at.elapsed().as_secs_f64() * 1_000.0;
    #[cfg(debug_assertions)]
    let missing_count = missing_ids.len();

    #[cfg(debug_assertions)]
    let missing_lookup_started_at = Instant::now();
    let missing_assets = state.database.images_by_ids(&missing_ids)?;
    #[cfg(debug_assertions)]
    let missing_lookup_ms = missing_lookup_started_at.elapsed().as_secs_f64() * 1_000.0;

    let mut candidates = lexical;
    candidates.extend(missing_assets);
    #[cfg(debug_assertions)]
    let candidate_count = candidates.len();

    #[cfg(debug_assertions)]
    let ranking_started_at = Instant::now();
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
    #[cfg(debug_assertions)]
    let ranking_ms = ranking_started_at.elapsed().as_secs_f64() * 1_000.0;

    #[cfg(debug_assertions)]
    let sort_started_at = Instant::now();
    ranked.sort_by(|(left_asset, left_score), (right_asset, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| right_asset.modified_at.cmp(&left_asset.modified_at))
    });
    let page = ranked
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();

    #[cfg(debug_assertions)]
    let top_results = page
        .iter()
        .take(8)
        .enumerate()
        .map(|(index, (asset, score))| {
            format!(
                "{}:{}:{}:{score:.4}",
                offset + index + 1,
                asset.id,
                asset.name,
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    let items = page
        .into_iter()
        .map(|(asset, _)| asset)
        .collect::<Vec<_>>();
    #[cfg(debug_assertions)]
    let sort_ms = sort_started_at.elapsed().as_secs_f64() * 1_000.0;

    #[cfg(debug_assertions)]
    tracing::event(
        "search.pipeline",
        format!(
            "id={} mode={mode} query={query:?} folder_id={folder_id:?} requested_limit={requested_limit} effective_limit={limit} offset={offset} window_end={window_end} candidate_limit={candidate_limit} total_available={total} query_vector_dimensions={} normalize_ms={normalize_ms:.2} fts_prepare_ms={fts_prepare_ms:.2} count_ms={count_ms:.2} lexical_db_ms={lexical_ms:.2} lexical_candidates={lexical_count} vector_lock_wait_ms={vector_lock_wait_ms:.2} vector_scan_ms={vector_scan_ms:.2} vector_store_len={vector_store_len} semantic_candidates={semantic_count} rank_maps_ms={rank_maps_ms:.2} missing_semantic_ids={missing_count} missing_lookup_ms={missing_lookup_ms:.2} merged_candidates={candidate_count} ranking_ms={ranking_ms:.2} sort_page_ms={sort_ms:.2} results={} total_ms={:.2} top_results={top_results:?}",
            diagnostic_id.unwrap_or("-"),
            query_vector.map_or(0, |vector| vector.len()),
            items.len(),
            total_started_at.elapsed().as_secs_f64() * 1_000.0,
        ),
    );

    Ok(SearchPage { items, total })
}
