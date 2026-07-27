use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

use crate::{
    AppError,
    models::{FollowedFolder, ImageAsset},
    state::AppState,
    tracing,
};

use super::{
    files::{discover_images, is_changed, prepare_asset},
    progress,
};

const METADATA_BATCH_SIZE: usize = 64;
const MAX_PENDING_IMAGES: usize = 20_000;

pub fn index_folder(
    state: &AppState,
    app: &AppHandle,
    folder: &FollowedFolder,
) -> Result<(), AppError> {
    let _trace = tracing::span("indexer.folder");
    let _guard = state.lock_indexer();
    progress::emit(app, folder, 0, 0, "discovering", "Analyse du dossier…");

    let paths = discover_images(Path::new(&folder.path));
    let current_paths = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<HashSet<_>>();
    let existing = state.database.fingerprints_for_folder(&folder.id)?;
    let changed = paths
        .into_iter()
        .filter(|path| is_changed(path, &existing))
        .collect::<Vec<PathBuf>>();

    if !changed.is_empty() {
        let total = changed.len();
        progress::emit(app, folder, 0, total, "metadata", "Lecture des métadonnées…");
        let mut processed = 0;
        for batch in changed.chunks(METADATA_BATCH_SIZE) {
            let prepared = batch
                .par_iter()
                .filter_map(|path| prepare_asset(folder, path).ok())
                .collect::<Vec<ImageAsset>>();
            if !prepared.is_empty() {
                let invalidated = prepared
                    .iter()
                    .map(|asset| asset.id.as_str())
                    .collect::<Vec<_>>();
                state.database.save_assets(&prepared, &[])?;
                state.vectors.write().remove_ids(invalidated);
            }
            processed = (processed + batch.len()).min(total);
            progress::emit(
                app,
                folder,
                processed,
                total,
                "metadata",
                &format!("Métadonnées · {processed} sur {total}"),
            );
        }
    }

    let deleted = state.database.delete_missing(&folder.id, &current_paths)?;
    state
        .vectors
        .write()
        .remove_ids(deleted.iter().map(String::as_str));

    let pending = pending_assets(state, Some(&folder.id))?;
    let _ = app.emit("library-updated", ());
    let _ = app.emit("semantic-index-requested", folder.id.clone());

    if pending.is_empty() {
        progress::emit(app, folder, 0, 0, "complete", "Bibliothèque à jour");
    } else {
        progress::emit(
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
    state.database.pending_images(folder_id, MAX_PENDING_IMAGES)
}
