use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

#[cfg(debug_assertions)]
use std::time::Instant;

use rayon::prelude::*;

use crate::tracing;

use super::{VectorMatch, VectorStore, maintenance::normalize_in_place};

impl VectorStore {
    pub fn top_k(
        &self,
        query: &[f32],
        folder_id: Option<&str>,
        limit: usize,
    ) -> Vec<VectorMatch> {
        self.top_k_with_diagnostics(query, folder_id, limit, None)
    }

    pub fn top_k_with_diagnostics(
        &self,
        query: &[f32],
        folder_id: Option<&str>,
        limit: usize,
        diagnostic_id: Option<&str>,
    ) -> Vec<VectorMatch> {
        #[cfg(debug_assertions)]
        let total_started_at = Instant::now();

        if limit == 0 || query.len() != self.dimensions || self.dimensions == 0 {
            #[cfg(debug_assertions)]
            tracing::event(
                "search.vector",
                format!(
                    "id={} status=skipped reason=invalid_input query_dimensions={} store_dimensions={} vectors={} limit={limit} folder_id={folder_id:?}",
                    diagnostic_id.unwrap_or("-"),
                    query.len(),
                    self.dimensions,
                    self.entries.len(),
                ),
            );
            return Vec::new();
        }

        #[cfg(debug_assertions)]
        let normalize_started_at = Instant::now();
        let mut normalized_query = query.to_vec();
        normalize_in_place(&mut normalized_query);
        #[cfg(debug_assertions)]
        let normalize_ms = normalize_started_at.elapsed().as_secs_f64() * 1_000.0;

        if normalized_query
            .iter()
            .all(|value| value.abs() <= f32::EPSILON)
        {
            #[cfg(debug_assertions)]
            tracing::event(
                "search.vector",
                format!(
                    "id={} status=skipped reason=zero_vector query_dimensions={} vectors={} normalize_ms={normalize_ms:.2} total_ms={:.2}",
                    diagnostic_id.unwrap_or("-"),
                    query.len(),
                    self.entries.len(),
                    total_started_at.elapsed().as_secs_f64() * 1_000.0,
                ),
            );
            return Vec::new();
        }

        let dimensions = self.dimensions;
        #[cfg(debug_assertions)]
        let scan_started_at = Instant::now();
        let heap = self
            .vectors
            .par_chunks_exact(dimensions)
            .zip(self.entries.par_iter())
            .enumerate()
            .filter(|(_, (_, metadata))| {
                folder_id.is_none_or(|folder_id| metadata.folder_id == folder_id)
            })
            .fold(
                || BinaryHeap::with_capacity(limit.saturating_add(1)),
                |mut heap, (index, (vector, _))| {
                    let cosine = dot_product(&normalized_query, vector);
                    heap.push(Reverse(Candidate {
                        index,
                        score: cosine,
                    }));
                    if heap.len() > limit {
                        heap.pop();
                    }
                    heap
                },
            )
            .reduce(
                || BinaryHeap::with_capacity(limit.saturating_add(1)),
                |mut left, right| {
                    for candidate in right {
                        left.push(candidate);
                        if left.len() > limit {
                            left.pop();
                        }
                    }
                    left
                },
            );
        #[cfg(debug_assertions)]
        let parallel_scan_ms = scan_started_at.elapsed().as_secs_f64() * 1_000.0;

        #[cfg(debug_assertions)]
        let materialize_started_at = Instant::now();
        let mut matches = heap
            .into_iter()
            .map(|Reverse(candidate)| VectorMatch {
                image_id: self.entries[candidate.index].image_id.clone(),
                score: ((candidate.score + 1.0) / 2.0).clamp(0.0, 1.0),
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.score.total_cmp(&left.score));
        #[cfg(debug_assertions)]
        let materialize_sort_ms = materialize_started_at.elapsed().as_secs_f64() * 1_000.0;

        #[cfg(debug_assertions)]
        tracing::event(
            "search.vector",
            format!(
                "id={} status=complete query_dimensions={} store_dimensions={} vectors={} folder_id={folder_id:?} requested_results={limit} results={} normalize_ms={normalize_ms:.2} parallel_scan_ms={parallel_scan_ms:.2} materialize_sort_ms={materialize_sort_ms:.2} best_score={:.4} total_ms={:.2}",
                diagnostic_id.unwrap_or("-"),
                query.len(),
                self.dimensions,
                self.entries.len(),
                matches.len(),
                matches.first().map_or(0.0, |item| item.score),
                total_started_at.elapsed().as_secs_f64() * 1_000.0,
            ),
        );

        matches
    }
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    index: usize,
    score: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.score.to_bits() == other.score.to_bits()
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| self.index.cmp(&other.index))
    }
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return 0.0;
    }
    dot_product(left, right) / (left_norm * right_norm)
}

fn dot_product(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}
