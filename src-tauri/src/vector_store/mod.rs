mod maintenance;
mod search;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::models::VectorEntry;

#[derive(Debug, Clone)]
pub(super) struct VectorMetadata {
    pub image_id: String,
    pub folder_id: String,
}

#[derive(Debug, Clone)]
pub struct VectorMatch {
    pub image_id: String,
    pub score: f32,
}

#[derive(Debug, Default)]
pub struct VectorStore {
    pub(super) dimensions: usize,
    pub(super) vectors: Vec<f32>,
    pub(super) entries: Vec<VectorMetadata>,
    pub(super) positions: HashMap<String, usize>,
}

impl VectorStore {
    pub fn from_entries(entries: Vec<VectorEntry>) -> Self {
        let mut store = Self::default();
        store.upsert(
            entries
                .into_iter()
                .map(|entry| (entry.image_id, entry.folder_id, entry.vector)),
        );
        store
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.dimensions = 0;
        self.vectors.clear();
        self.entries.clear();
        self.positions.clear();
    }

    pub fn vector(&self, image_id: &str) -> Option<&[f32]> {
        let position = *self.positions.get(image_id)?;
        let start = position * self.dimensions;
        Some(&self.vectors[start..start + self.dimensions])
    }
}

pub use search::cosine_similarity;
