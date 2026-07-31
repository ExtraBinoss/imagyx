use std::collections::{HashMap, HashSet};

use super::normalize_query;

#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub image_id: String,
    pub score: f32,
}

#[derive(Debug, Default)]
pub struct FuzzyIndex {
    records: HashMap<String, FuzzyRecord>,
    token_ids: HashMap<String, HashSet<String>>,
    buckets: HashMap<(usize, char), HashSet<String>>,
}

#[derive(Debug, Clone)]
struct FuzzyRecord {
    folder_id: String,
    tokens: Vec<String>,
}

impl FuzzyIndex {
    pub fn from_entries(entries: Vec<(String, String, String)>) -> Self {
        let mut index = Self::default();
        for (image_id, folder_id, name) in entries {
            index.upsert(image_id, folder_id, &name);
        }
        index
    }

    pub fn upsert(&mut self, image_id: String, folder_id: String, name: &str) {
        self.remove(&image_id);
        let tokens = normalize_query(name)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for token in &tokens {
            self.token_ids
                .entry(token.clone())
                .or_default()
                .insert(image_id.clone());
            if let Some(first) = token.chars().next() {
                self.buckets
                    .entry((token.chars().count(), first))
                    .or_default()
                    .insert(token.clone());
            }
        }
        self.records
            .insert(image_id, FuzzyRecord { folder_id, tokens });
    }

    pub fn remove(&mut self, image_id: &str) {
        let Some(record) = self.records.remove(image_id) else {
            return;
        };
        for token in record.tokens {
            let mut remove_token = false;
            if let Some(ids) = self.token_ids.get_mut(&token) {
                ids.remove(image_id);
                remove_token = ids.is_empty();
            }
            if remove_token {
                self.token_ids.remove(&token);
                if let Some(first) = token.chars().next() {
                    let key = (token.chars().count(), first);
                    if let Some(tokens) = self.buckets.get_mut(&key) {
                        tokens.remove(&token);
                        if tokens.is_empty() {
                            self.buckets.remove(&key);
                        }
                    }
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

    pub fn search(
        &self,
        query_tokens: &[String],
        folder_id: Option<&str>,
        limit: usize,
    ) -> Vec<FuzzyMatch> {
        if query_tokens.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scores: HashMap<&str, f32> = HashMap::new();
        for query in query_tokens {
            let length = query.chars().count();
            let Some(first) = query.chars().next() else {
                continue;
            };
            let mut candidate_tokens = Vec::new();
            for candidate_length in length.saturating_sub(1)..=length.saturating_add(1) {
                if let Some(tokens) = self.buckets.get(&(candidate_length, first)) {
                    candidate_tokens.extend(tokens.iter());
                }
            }
            for candidate in candidate_tokens {
                let distance = damerau_distance_at_most_one(query, candidate);
                if distance > 1 {
                    continue;
                }
                let token_score =
                    1.0 - distance as f32 / length.max(candidate.chars().count()) as f32;
                if let Some(ids) = self.token_ids.get(candidate) {
                    for image_id in ids {
                        let Some(record) = self.records.get(image_id) else {
                            continue;
                        };
                        if folder_id.is_some_and(|folder| record.folder_id != folder) {
                            continue;
                        }
                        scores
                            .entry(image_id.as_str())
                            .and_modify(|score| *score = score.max(token_score))
                            .or_insert(token_score);
                    }
                }
            }
        }
        let mut matches = scores
            .into_iter()
            .map(|(image_id, score)| FuzzyMatch {
                image_id: image_id.to_owned(),
                score,
            })
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

fn damerau_distance_at_most_one(left: &str, right: &str) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    if left == right {
        return 0;
    }
    if left.len().abs_diff(right.len()) > 1 {
        return 2;
    }
    if left.len() == right.len() {
        let differences = left
            .iter()
            .zip(&right)
            .enumerate()
            .filter_map(|(index, (a, b))| (a != b).then_some(index))
            .collect::<Vec<_>>();
        if differences.len() == 2 {
            let first = differences[0];
            let second = differences[1];
            if second == first + 1 && left[first] == right[second] && left[second] == right[first] {
                return 1;
            }
        }
    }
    let mut edits = 0;
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        if left[left_index] == right[right_index] {
            left_index += 1;
            right_index += 1;
        } else {
            edits += 1;
            if edits > 1 {
                return 2;
            }
            if left.len() > right.len() {
                left_index += 1;
            } else if right.len() > left.len() {
                right_index += 1;
            } else {
                left_index += 1;
                right_index += 1;
            }
        }
    }
    edits + (left.len() - left_index) + (right.len() - right_index)
}

#[cfg(test)]
mod tests {
    use super::FuzzyIndex;

    #[test]
    fn finds_a_transposed_filename_token_without_substring_false_positives() {
        let index = FuzzyIndex::from_entries(vec![
            ("woman".into(), "folder".into(), "woman.jpg".into()),
            ("cartoon".into(), "folder".into(), "cartoon.jpg".into()),
        ]);
        let matches = index.search(&vec!["womna".into()], None, 20);
        assert_eq!(
            matches.first().map(|item| item.image_id.as_str()),
            Some("woman")
        );
        assert!(!matches.iter().any(|item| item.image_id == "cartoon"));
    }

    #[test]
    fn returns_exact_alias_candidates_for_human_queries() {
        let index = FuzzyIndex::from_entries(vec![
            ("girl".into(), "folder".into(), "girl_train_segmented.png".into()),
            ("cartoon".into(), "folder".into(), "cartoon.png".into()),
        ]);
        let matches = index.search(&vec!["girl".into()], None, 20);

        assert_eq!(matches.first().map(|item| item.image_id.as_str()), Some("girl"));
        assert_eq!(matches.first().map(|item| item.score), Some(1.0));
    }
}
