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
    models::{FollowedFolder, ImageAsset, IndexProgress},
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
    let pending = pending_assets(state, Some(&folder.id))?;
    let _ = app.emit("library-updated", ());
    let _ = app.emit("semantic-index-requested", folder.id.clone());

    if pending.is_empty() {
        emit_progress(app, folder, 0, 0, "complete", "Bibliothèque à jour");
    } else {
        emit_progress(
            app,
            folder,
            0,
            pending.len(),
            "queued",
            &format!("{} images visibles · analyse WebGPU en attente", pending.len()),
        );
    }
    Ok(())
}

pub fn pending_assets(
    state: &AppState,
    folder_id: Option<&str>,
) -> Result<Vec<ImageAsset>, AppError> {
    let vector_ids: HashSet<String> = state
        .vectors
        .read()
        .iter()
        .filter(|entry| folder_id.is_none_or(|folder_id| entry.folder_id == folder_id))
        .map(|entry| entry.image_id.clone())
        .collect();
    Ok(state
        .database
        .images(folder_id)?
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
    query_vector: Option<&[f32]>,
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
    let semantic_scores = semantic_scores(state, query_vector, folder_id);

    assets.retain_mut(|asset| {
        let lexical = lexical_score(asset, &tokens);
        let semantic = semantic_scores.get(&asset.id).copied();
        let score = semantic.map_or(lexical, |value| 0.88 * value + 0.12 * lexical);
        asset.semantic_score = semantic.map(|_| score.clamp(0.0, 1.0));
        score > 0.04
    });
    assets.sort_by(|left, right| {
        let left_score = left.semantic_score.unwrap_or_else(|| lexical_score(left, &tokens));
        let right_score = right.semantic_score.unwrap_or_else(|| lexical_score(right, &tokens));
        right_score
            .total_cmp(&left_score)
            .then_with(|| right.modified_at.cmp(&left.modified_at))
    });
    assets.truncate(limit);
    Ok(assets)
}

fn semantic_scores(
    state: &AppState,
    query_vector: Option<&[f32]>,
    folder_id: Option<&str>,
) -> HashMap<String, f32> {
    let Some(query_vector) = query_vector else {
        return HashMap::new();
    };
    state
        .vectors
        .read()
        .par_iter()
        .filter(|entry| folder_id.is_none_or(|folder_id| entry.folder_id == folder_id))
        .filter(|entry| entry.vector.len() == query_vector.len())
        .map(|entry| {
            let cosine = cosine_similarity(query_vector, &entry.vector);
            (entry.image_id.clone(), ((cosine + 1.0) / 2.0).clamp(0.0, 1.0))
        })
        .collect()
}

fn lexical_score(asset: &ImageAsset, tokens: &[&str]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let haystack = format!("{} {}", asset.name.to_lowercase(), asset.path.to_lowercase());
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
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (left_norm * right_norm)
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
