use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Instant, UNIX_EPOCH},
};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::{
    AppError,
    models::{FollowedFolder, ImageAsset, IndexProgress, ModelStatus},
    state::AppState,
};

pub fn index_folder(
    state: &AppState,
    app: &AppHandle,
    folder: &FollowedFolder,
) -> Result<(), AppError> {
    let _guard = state.lock_indexer();
    emit_progress(app, folder, 0, 0, "discovering", "Analyse du dossier…");
    let paths = discover_images(Path::new(&folder.path));
    let current_paths: HashSet<String> = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let existing: HashMap<String, (i64, u64)> = state
        .database
        .images(Some(&folder.id))?
        .into_iter()
        .map(|asset| (asset.path, (asset.modified_at, asset.size_bytes)))
        .collect();
    let changed: Vec<PathBuf> = paths
        .into_iter()
        .filter(|path| is_changed(path, &existing))
        .collect();

    if !changed.is_empty() {
        emit_progress(
            app,
            folder,
            0,
            changed.len(),
            "metadata",
            "Lecture des métadonnées…",
        );
        let prepared: Vec<ImageAsset> = changed
            .par_iter()
            .filter_map(|path| prepare_asset(folder, path).ok())
            .collect();
        state.database.save_assets(&prepared, &[])?;
    }

    state.database.delete_missing(&folder.id, &current_paths)?;
    state.refresh_vectors()?;
    let _ = app.emit("library-updated", ());

    let pending = pending_assets(state, folder)?;
    if pending.is_empty() {
        emit_progress(app, folder, 0, 0, "complete", "Bibliothèque à jour");
        return Ok(());
    }
    if state.model_progress.read().stage != "ready" {
        emit_progress(
            app,
            folder,
            0,
            pending.len(),
            "queued",
            &format!("{} images visibles · analyse IA en attente", pending.len()),
        );
        return Ok(());
    }
    embed_pending_locked(state, app, folder, &pending)
}

pub fn embed_pending(
    state: &AppState,
    app: &AppHandle,
    folder: &FollowedFolder,
) -> Result<(), AppError> {
    let _guard = state.lock_indexer();
    let pending = pending_assets(state, folder)?;
    if pending.is_empty() || state.model_progress.read().stage != "ready" {
        return Ok(());
    }
    embed_pending_locked(state, app, folder, &pending)
}

fn embed_pending_locked(
    state: &AppState,
    app: &AppHandle,
    folder: &FollowedFolder,
    pending: &[ImageAsset],
) -> Result<(), AppError> {
    let maximum_batch_size = state.ml.lock().batch_size().clamp(4, 16);
    let started = Instant::now();
    let mut processed = 0;
    let mut batch_index = 0;
    let mut next_batch_size = 4.min(pending.len().max(1));

    {
        let mut stats = state.runtime_stats.write();
        stats.stage = "indexing".to_owned();
        stats.current = 0;
        stats.total = pending.len();
        stats.batch_size = next_batch_size;
        stats.batch_current = 0;
        stats.batch_total = pending.len().div_ceil(next_batch_size);
        stats.elapsed_ms = 0;
        stats.images_per_second = 0.0;
        stats.average_ms_per_image = 0.0;
        stats.decode_ms = 0;
        stats.inference_ms = 0;
        stats.save_ms = 0;
    }

    while processed < pending.len() {
        let end = (processed + next_batch_size).min(pending.len());
        let chunk = &pending[processed..end];
        batch_index += 1;
        let estimated_total_batches = batch_index + (pending.len() - processed).div_ceil(next_batch_size) - 1;

        emit_batch_progress(
            app,
            folder,
            processed,
            pending.len(),
            batch_index,
            estimated_total_batches,
            &format!("Analyse IA · décodage de {} images", chunk.len()),
        );

        let paths: Vec<PathBuf> = chunk
            .iter()
            .map(|asset| PathBuf::from(&asset.path))
            .collect();

        let batch_started = Instant::now();
        let embedding_batch = state.ml.lock().embed_images(&paths)?;

        emit_batch_progress(
            app,
            folder,
            processed,
            pending.len(),
            batch_index,
            estimated_total_batches,
            "Analyse IA · sauvegarde des résultats…",
        );

        let save_started = Instant::now();
        let embeddings: Vec<(String, Vec<f32>)> = chunk
            .iter()
            .zip(embedding_batch.vectors)
            .map(|(asset, vector)| (asset.id.clone(), vector))
            .collect();
        state.database.save_assets(chunk, &embeddings)?;
        let save_ms = save_started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

        processed = end;
        let elapsed = started.elapsed();
        let elapsed_seconds = elapsed.as_secs_f32().max(0.001);
        let batch_ms = batch_started.elapsed().as_secs_f32() * 1000.0;
        let ms_per_image = batch_ms / chunk.len().max(1) as f32;

        if ms_per_image < 140.0 && next_batch_size < maximum_batch_size {
            next_batch_size = (next_batch_size * 2).min(maximum_batch_size);
        } else if ms_per_image > 500.0 && next_batch_size > 4 {
            next_batch_size = (next_batch_size / 2).max(4);
        }

        {
            let mut stats = state.runtime_stats.write();
            stats.current = processed;
            stats.batch_size = next_batch_size;
            stats.batch_current = batch_index;
            stats.batch_total = batch_index + (pending.len() - processed).div_ceil(next_batch_size);
            stats.elapsed_ms = elapsed.as_millis().try_into().unwrap_or(u64::MAX);
            stats.images_per_second = processed as f32 / elapsed_seconds;
            stats.average_ms_per_image = elapsed.as_secs_f32() * 1000.0 / processed as f32;
            stats.decode_ms = stats.decode_ms.saturating_add(embedding_batch.decode_ms);
            stats.inference_ms = stats.inference_ms.saturating_add(embedding_batch.inference_ms);
            stats.save_ms = stats.save_ms.saturating_add(save_ms);
        }

        emit_batch_progress(
            app,
            folder,
            processed,
            pending.len(),
            batch_index,
            batch_index + (pending.len() - processed).div_ceil(next_batch_size),
            &format!(
                "Analyse IA · {processed} sur {} · {:.1} img/s",
                pending.len(),
                processed as f32 / elapsed_seconds
            ),
        );
    }

    state.refresh_vectors()?;
    let status = state.ml.lock().status();
    state.runtime_stats.write().stage = "ready".to_owned();
    let _ = app.emit("model-status", status);
    emit_progress(
        app,
        folder,
        pending.len(),
        pending.len(),
        "complete",
        "Analyse IA terminée",
    );
    let _ = app.emit("library-updated", ());
    Ok(())
}

fn pending_assets(state: &AppState, folder: &FollowedFolder) -> Result<Vec<ImageAsset>, AppError> {
    let vector_ids: HashSet<String> = state
        .vectors
        .read()
        .iter()
        .filter(|entry| entry.folder_id == folder.id)
        .map(|entry| entry.image_id.clone())
        .collect();
    Ok(state
        .database
        .images(Some(&folder.id))?
        .into_iter()
        .filter(|asset| !vector_ids.contains(&asset.id))
        .collect())
}

fn discover_images(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| is_supported_image(path))
        .collect()
}

fn is_changed(path: &Path, existing: &HashMap<String, (i64, u64)>) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    let fingerprint = (modified_millis(&metadata), metadata.len());
    existing
        .get(path.to_string_lossy().as_ref())
        .is_none_or(|existing| *existing != fingerprint)
}

fn prepare_asset(folder: &FollowedFolder, path: &Path) -> Result<ImageAsset, AppError> {
    let metadata = path.metadata()?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::InvalidPath(path.to_path_buf()))?
        .to_owned();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let path_string = path.to_string_lossy().into_owned();
    let id = blake3::hash(path_string.as_bytes()).to_hex().to_string();
    let (width, height) = image::image_dimensions(path)?;
    Ok(ImageAsset {
        id,
        folder_id: folder.id.clone(),
        path: path_string,
        name: file_name,
        extension,
        width,
        height,
        size_bytes: metadata.len(),
        modified_at: modified_millis(&metadata),
        thumbnail_path: String::new(),
        semantic_score: None,
    })
}

fn modified_millis(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "ico"
            )
        })
}

pub fn search(
    state: &AppState,
    query: &str,
    folder_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ImageAsset>, AppError> {
    let mut assets = state.database.images(folder_id)?;
    if query.trim().is_empty() {
        assets.truncate(limit);
        return Ok(assets);
    }

    let normalized = query.trim().to_lowercase();
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let semantic_scores = semantic_scores(state, query, folder_id).unwrap_or_default();

    assets.retain_mut(|asset| {
        let haystack = format!(
            "{} {}",
            asset.name.to_lowercase(),
            asset.path.to_lowercase()
        );
        let lexical_hits = tokens
            .iter()
            .filter(|token| haystack.contains(**token))
            .count();
        let lexical = if tokens.is_empty() {
            0.0
        } else {
            lexical_hits as f32 / tokens.len() as f32
        };
        let semantic = semantic_scores.get(&asset.id).copied();
        let score = match semantic {
            Some(value) => 0.82 * value + 0.18 * lexical,
            None => lexical,
        };
        asset.semantic_score = semantic.map(|_| score.clamp(0.0, 1.0));
        score > 0.04
    });

    assets.sort_by(|left, right| {
        let left_score = left
            .semantic_score
            .unwrap_or_else(|| lexical_score(left, &tokens));
        let right_score = right
            .semantic_score
            .unwrap_or_else(|| lexical_score(right, &tokens));
        right_score
            .total_cmp(&left_score)
            .then_with(|| right.modified_at.cmp(&left.modified_at))
    });
    assets.truncate(limit);
    Ok(assets)
}

fn semantic_scores(
    state: &AppState,
    query: &str,
    folder_id: Option<&str>,
) -> Result<HashMap<String, f32>, AppError> {
    if state.model_progress.read().stage != "ready" || state.vectors.read().is_empty() {
        return Ok(HashMap::new());
    }
    let query_vector = state.ml.lock().embed_text(query)?;
    let vectors = state.vectors.read();
    Ok(vectors
        .par_iter()
        .filter(|entry| folder_id.is_none_or(|folder_id| entry.folder_id == folder_id))
        .map(|entry| {
            let cosine = cosine_similarity(&query_vector, &entry.vector);
            (
                entry.image_id.clone(),
                ((cosine + 1.0) / 2.0).clamp(0.0, 1.0),
            )
        })
        .collect())
}

fn lexical_score(asset: &ImageAsset, tokens: &[&str]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let haystack = format!(
        "{} {}",
        asset.name.to_lowercase(),
        asset.path.to_lowercase()
    );
    tokens
        .iter()
        .filter(|token| haystack.contains(**token))
        .count() as f32
        / tokens.len() as f32
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn emit_progress(
    app: &AppHandle,
    folder: &FollowedFolder,
    current: usize,
    total: usize,
    stage: &str,
    message: &str,
) {
    let _ = app.emit(
        "index-progress",
        IndexProgress {
            folder_id: folder.id.clone(),
            folder_name: folder.name.clone(),
            current,
            total,
            batch_current: None,
            batch_total: None,
            stage: stage.to_owned(),
            message: message.to_owned(),
        },
    );
}

fn emit_batch_progress(
    app: &AppHandle,
    folder: &FollowedFolder,
    current: usize,
    total: usize,
    batch_current: usize,
    batch_total: usize,
    message: &str,
) {
    let _ = app.emit(
        "index-progress",
        IndexProgress {
            folder_id: folder.id.clone(),
            folder_name: folder.name.clone(),
            current,
            total,
            batch_current: Some(batch_current),
            batch_total: Some(batch_total),
            stage: "embedding".to_owned(),
            message: message.to_owned(),
        },
    );
}
