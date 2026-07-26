use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::{
    AppError,
    models::{FollowedFolder, ImageAsset, IndexProgress, ModelStatus},
    state::AppState,
};

const EMBEDDING_CHUNK_SIZE: usize = 8;

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

        emit_progress(
            app,
            folder,
            prepared.len(),
            prepared.len(),
            "saving",
            "Mise à jour de la bibliothèque…",
        );

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
            &format!(
                "{} image{} affichée{} · analyse IA dès que le modèle est prêt",
                pending.len(),
                if pending.len() > 1 { "s" } else { "" },
                if pending.len() > 1 { "s" } else { "" }
            ),
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
    if pending.is_empty() {
        return Ok(());
    }
    if state.model_progress.read().stage != "ready" {
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
    let total_batches = pending.len().div_ceil(EMBEDDING_CHUNK_SIZE);
    let mut processed = 0;

    for (batch_index, chunk) in pending.chunks(EMBEDDING_CHUNK_SIZE).enumerate() {
        emit_batch_progress(
            app,
            folder,
            processed,
            pending.len(),
            batch_index + 1,
            total_batches,
            &format!(
                "Analyse IA · lot {} sur {} · {} image{}",
                batch_index + 1,
                total_batches,
                chunk.len(),
                if chunk.len() > 1 { "s" } else { "" }
            ),
        );

        let paths: Vec<PathBuf> = chunk
            .iter()
            .map(|asset| PathBuf::from(&asset.path))
            .collect();

        match state.ml.lock().embed_images(&paths) {
            Ok(vectors) => {
                let embeddings: Vec<(String, Vec<f32>)> = chunk
                    .iter()
                    .zip(vectors)
                    .map(|(asset, vector)| (asset.id.clone(), vector))
                    .collect();
                state.database.save_assets(chunk, &embeddings)?;
            }
            Err(batch_error) => {
                eprintln!("Imagyx batch embedding failed, retrying individually: {batch_error}");
                embed_individually(state, chunk)?;
            }
        }

        processed += chunk.len();
        emit_batch_progress(
            app,
            folder,
            processed,
            pending.len(),
            batch_index + 1,
            total_batches,
            &format!("Analyse IA · {processed} sur {}", pending.len()),
        );
    }

    state.refresh_vectors()?;
    let _ = app.emit(
        "model-status",
        ModelStatus {
            ready: true,
            backend: crate::ml::MlRuntime::backend_label().to_owned(),
        },
    );
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

fn embed_individually(state: &AppState, assets: &[ImageAsset]) -> Result<(), AppError> {
    for asset in assets {
        let path = PathBuf::from(&asset.path);
        match state.ml.lock().embed_images(&[path]) {
            Ok(mut vectors) => {
                if let Some(vector) = vectors.pop() {
                    state
                        .database
                        .save_assets(std::slice::from_ref(asset), &[(asset.id.clone(), vector)])?;
                }
            }
            Err(error) => {
                eprintln!("Imagyx skipped {} during semantic indexing: {error}", asset.path);
            }
        }
    }
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
    let modified_at = modified_millis(&metadata);
    let size_bytes = metadata.len();
    let path_string = path.to_string_lossy();
    existing
        .get(path_string.as_ref())
        .is_none_or(|fingerprint| fingerprint.0 != modified_at || fingerprint.1 != size_bytes)
}

fn prepare_asset(folder: &FollowedFolder, path: &Path) -> Result<ImageAsset, AppError> {
    let metadata = path.metadata()?;
    let modified_at = modified_millis(&metadata);
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
        modified_at,
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
    let (dot, left_norm, right_norm) = left.iter().zip(right).fold(
        (0.0, 0.0, 0.0),
        |(dot, left_norm, right_norm), (left, right)| {
            (
                dot + left * right,
                left_norm + left * left,
                right_norm + right * right,
            )
        },
    );
    let denominator = left_norm.sqrt() * right_norm.sqrt();
    if denominator <= f32::EPSILON {
        0.0
    } else {
        dot / denominator
    }
}

fn emit_progress(
    app: &AppHandle,
    folder: &FollowedFolder,
    current: usize,
    total: usize,
    stage: &str,
    message: &str,
) {
    emit_progress_inner(app, folder, current, total, None, None, stage, message);
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
    emit_progress_inner(
        app,
        folder,
        current,
        total,
        Some(batch_current),
        Some(batch_total),
        "embedding",
        message,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_progress_inner(
    app: &AppHandle,
    folder: &FollowedFolder,
    current: usize,
    total: usize,
    batch_current: Option<usize>,
    batch_total: Option<usize>,
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
            batch_current,
            batch_total,
            stage: stage.to_owned(),
            message: message.to_owned(),
        },
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{cosine_similarity, is_supported_image};

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_supported_image(Path::new("poster.PNG")));
        assert!(is_supported_image(Path::new("photo.webp")));
        assert!(!is_supported_image(Path::new("notes.txt")));
    }

    #[test]
    fn cosine_similarity_handles_aligned_vectors() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 0.000_1);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }
}
