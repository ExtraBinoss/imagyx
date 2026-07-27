use tauri::{AppHandle, Emitter};

use crate::models::{FollowedFolder, IndexProgress};

pub(super) fn emit(
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
