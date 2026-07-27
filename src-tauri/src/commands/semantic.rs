use std::{collections::HashMap, path::Path, sync::Arc};

use rayon::prelude::*;
use tauri::State;

use crate::{
    indexer,
    models::{ImageEmbedding, ImageExplanation, QueryConcept, SemanticMatch},
    state::AppState,
};

#[tauri::command(rename_all = "camelCase")]
pub async fn prepare_ai_images(
    image_ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let by_id = state
            .database
            .images_by_ids(&image_ids)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|image| (image.id.clone(), image))
            .collect::<HashMap<_, _>>();
        image_ids
            .par_iter()
            .map(|image_id| {
                let image = by_id
                    .get(image_id)
                    .ok_or_else(|| format!("Image inconnue: {image_id}"))?;
                state
                    .thumbnails
                    .get_or_create(&image.id, Path::new(&image.path), image.modified_at)
                    .map(|path| path.to_string_lossy().into_owned())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn save_embeddings(
    embeddings: Vec<ImageEmbedding>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let rows = embeddings
        .into_iter()
        .map(|entry| (entry.image_id, entry.vector))
        .collect::<Vec<_>>();
    let image_ids = rows
        .iter()
        .map(|(image_id, _)| image_id.clone())
        .collect::<Vec<_>>();
    let folder_ids = state
        .database
        .folder_ids_for_images(&image_ids)
        .map_err(|error| error.to_string())?;
    state
        .database
        .save_embeddings(&rows)
        .map_err(|error| error.to_string())?;
    state.vectors.write().upsert(rows.into_iter().filter_map(
        |(image_id, vector)| {
            let folder_id = folder_ids.get(&image_id)?.clone();
            Some((image_id, folder_id, vector))
        },
    ));
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn explain_results(
    image_ids: Vec<String>,
    concepts: Vec<QueryConcept>,
    state: State<'_, Arc<AppState>>,
) -> Vec<ImageExplanation> {
    if image_ids.is_empty() || concepts.is_empty() {
        return Vec::new();
    }

    let vectors = state.vectors.read();
    image_ids
        .iter()
        .filter_map(|image_id| {
            let vector = vectors.vector(image_id)?;
            let mut matches = concepts
                .iter()
                .filter(|concept| concept.vector.len() == vector.len())
                .map(|concept| SemanticMatch {
                    label: concept.label.clone(),
                    score: ((indexer::cosine_similarity(&concept.vector, vector) + 1.0) / 2.0)
                        .clamp(0.0, 1.0),
                    source: "semantic".into(),
                })
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| right.score.total_cmp(&left.score));
            matches.truncate(8);
            Some(ImageExplanation {
                image_id: image_id.clone(),
                matches,
            })
        })
        .collect()
}

#[tauri::command(rename_all = "camelCase")]
pub fn top_image_tags(
    concepts: Vec<QueryConcept>,
    limit: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Vec<String> {
    if concepts.is_empty() {
        return Vec::new();
    }
    let vectors = state.vectors.read().snapshot(2_000);
    let counts = vectors
        .par_iter()
        .filter_map(|entry| {
            concepts
                .iter()
                .filter(|concept| concept.vector.len() == entry.vector.len())
                .map(|concept| {
                    (
                        concept.label.clone(),
                        indexer::cosine_similarity(&concept.vector, &entry.vector),
                    )
                })
                .max_by(|left, right| left.1.total_cmp(&right.1))
                .map(|best| best.0)
        })
        .fold(HashMap::<String, usize>::new, |mut map, label| {
            *map.entry(label).or_default() += 1;
            map
        })
        .reduce(HashMap::<String, usize>::new, |mut left, right| {
            for (label, count) in right {
                *left.entry(label).or_default() += count;
            }
            left
        });
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_label, left_count), (right_label, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_label.cmp(right_label))
    });
    ranked.truncate(limit.unwrap_or(10).clamp(1, 20));
    ranked.into_iter().map(|(label, _)| label).collect()
}
