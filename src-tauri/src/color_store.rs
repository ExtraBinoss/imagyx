use std::collections::{HashMap, HashSet};

use crate::fuzzy::{ColorKind, score_signature};

#[derive(Debug, Clone)]
pub struct ColorMatch {
    pub image_id: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
struct ColorRecord {
    folder_id: String,
    signature: Vec<u8>,
    bucket_keys: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct ColorStore {
    records: HashMap<String, ColorRecord>,
    buckets: HashMap<u8, HashSet<String>>,
}

impl ColorStore {
    pub fn from_entries(entries: Vec<(String, String, Vec<u8>)>) -> Self {
        let mut store = Self::default();
        for (image_id, folder_id, signature) in entries {
            store.upsert(image_id, folder_id, signature);
        }
        store
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn upsert(&mut self, image_id: String, folder_id: String, signature: Vec<u8>) {
        self.remove(&image_id);
        if signature.len() < 18 {
            return;
        }
        let bucket_keys = [
            ColorKind::Red,
            ColorKind::Orange,
            ColorKind::Yellow,
            ColorKind::Green,
            ColorKind::Cyan,
            ColorKind::Blue,
            ColorKind::Purple,
            ColorKind::Pink,
            ColorKind::Black,
            ColorKind::Gray,
            ColorKind::White,
        ]
        .into_iter()
        .filter(|color| score_signature(&signature, &[*color], false) > 0.03)
        .map(|color| color as u8)
        .collect::<Vec<_>>();
        for key in &bucket_keys {
            self.buckets
                .entry(*key)
                .or_default()
                .insert(image_id.clone());
        }
        self.records.insert(
            image_id,
            ColorRecord {
                folder_id,
                signature,
                bucket_keys,
            },
        );
    }

    pub fn remove(&mut self, image_id: &str) {
        let Some(record) = self.records.remove(image_id) else {
            return;
        };
        for key in record.bucket_keys {
            if let Some(bucket) = self.buckets.get_mut(&key) {
                bucket.remove(image_id);
                if bucket.is_empty() {
                    self.buckets.remove(&key);
                }
            }
        }
    }

    pub fn remove_ids<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) {
        for id in ids {
            self.remove(id);
        }
    }

    pub fn remove_folder(&mut self, folder_id: &str) {
        let ids = self
            .records
            .iter()
            .filter(|(_, record)| record.folder_id == folder_id)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        self.remove_ids(ids.iter().map(String::as_str));
    }

    pub fn top_k(
        &self,
        colors: &[ColorKind],
        dominant: bool,
        folder_id: Option<&str>,
        limit: usize,
    ) -> Vec<ColorMatch> {
        if colors.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut candidate_ids = HashSet::new();
        for color in colors {
            if let Some(ids) = self.buckets.get(&(*color as u8)) {
                candidate_ids.extend(ids.iter().cloned());
            }
        }
        let mut matches = candidate_ids
            .into_iter()
            .filter_map(|image_id| self.records.get(&image_id).map(|record| (image_id, record)))
            .filter(|(_, record)| folder_id.is_none_or(|folder| record.folder_id == folder))
            .map(|(image_id, record)| ColorMatch {
                image_id,
                score: score_signature(&record.signature, colors, dominant),
            })
            .filter(|item| item.score > 0.01)
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.image_id.cmp(&right.image_id))
        });
        matches.truncate(limit);
        matches
    }
}
