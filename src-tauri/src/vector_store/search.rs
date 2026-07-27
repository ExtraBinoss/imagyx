use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

use rayon::prelude::*;

use super::{VectorMatch, VectorStore, maintenance::normalize_in_place};

impl VectorStore {
    pub fn top_k(
        &self,
        query: &[f32],
        folder_id: Option<&str>,
        limit: usize,
    ) -> Vec<VectorMatch> {
        if limit == 0 || query.len() != self.dimensions || self.dimensions == 0 {
            return Vec::new();
        }
        let mut normalized_query = query.to_vec();
        normalize_in_place(&mut normalized_query);
        if normalized_query
            .iter()
            .all(|value| value.abs() <= f32::EPSILON)
        {
            return Vec::new();
        }

        let dimensions = self.dimensions;
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

        let mut matches = heap
            .into_iter()
            .map(|Reverse(candidate)| VectorMatch {
                image_id: self.entries[candidate.index].image_id.clone(),
                score: ((candidate.score + 1.0) / 2.0).clamp(0.0, 1.0),
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.score.total_cmp(&left.score));
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
