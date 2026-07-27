use std::collections::{HashMap, HashSet};

use crate::models::VectorEntry;

use super::{VectorMetadata, VectorStore};

impl VectorStore {
    pub fn upsert<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = (String, String, Vec<f32>)>,
    {
        for (image_id, folder_id, mut vector) in entries {
            if vector.is_empty() {
                continue;
            }
            normalize_in_place(&mut vector);
            if self.dimensions == 0 {
                self.dimensions = vector.len();
            }
            if vector.len() != self.dimensions {
                continue;
            }

            if let Some(&position) = self.positions.get(&image_id) {
                let start = position * self.dimensions;
                self.vectors[start..start + self.dimensions].copy_from_slice(&vector);
                self.entries[position].folder_id = folder_id;
            } else {
                let position = self.entries.len();
                self.positions.insert(image_id.clone(), position);
                self.entries.push(VectorMetadata {
                    image_id,
                    folder_id,
                });
                self.vectors.extend_from_slice(&vector);
            }
        }
    }

    pub fn remove_ids<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) {
        let removed = ids.into_iter().collect::<HashSet<_>>();
        if removed.is_empty() {
            return;
        }
        self.retain(|entry| !removed.contains(entry.image_id.as_str()));
    }

    pub fn remove_folder(&mut self, folder_id: &str) {
        self.retain(|entry| entry.folder_id != folder_id);
    }

    pub fn snapshot(&self, limit: usize) -> Vec<VectorEntry> {
        self.entries
            .iter()
            .take(limit)
            .enumerate()
            .map(|(position, metadata)| {
                let start = position * self.dimensions;
                VectorEntry {
                    image_id: metadata.image_id.clone(),
                    folder_id: metadata.folder_id.clone(),
                    vector: self.vectors[start..start + self.dimensions].to_vec(),
                }
            })
            .collect()
    }

    fn retain(&mut self, mut keep: impl FnMut(&VectorMetadata) -> bool) {
        if self.entries.is_empty() {
            return;
        }
        let mut entries = Vec::with_capacity(self.entries.len());
        let mut vectors = Vec::with_capacity(self.vectors.len());
        let mut positions = HashMap::with_capacity(self.positions.len());

        for (position, metadata) in self.entries.iter().enumerate() {
            if !keep(metadata) {
                continue;
            }
            let next_position = entries.len();
            positions.insert(metadata.image_id.clone(), next_position);
            entries.push(metadata.clone());
            let start = position * self.dimensions;
            vectors.extend_from_slice(&self.vectors[start..start + self.dimensions]);
        }

        self.entries = entries;
        self.vectors = vectors;
        self.positions = positions;
        if self.entries.is_empty() {
            self.dimensions = 0;
        }
    }
}

pub(super) fn normalize_in_place(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        vector.fill(0.0);
        return;
    }
    for value in vector {
        *value /= norm;
    }
}
