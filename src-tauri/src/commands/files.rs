use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use arboard::{Clipboard, ImageData};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::AppState;

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_file_manager(
    path: String,
    reveal: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let canonical = managed_path(&state, &path)?;
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        if reveal && canonical.is_file() {
            command.arg(format!("/select,{}", canonical.display()));
        } else {
            command.arg(&canonical);
        }
        command
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir Explorer: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if reveal && canonical.is_file() {
            command.arg("-R");
        }
        command.arg(&canonical);
        command
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir Finder: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if canonical.is_file() {
            canonical.parent().unwrap_or(&canonical)
        } else {
            canonical.as_path()
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir le gestionnaire de fichiers: {error}"))?;
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copy_image_to_clipboard(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let canonical = managed_path(&state, &path)?;
        if !canonical.is_file() {
            return Err("Le chemin ne correspond pas à une image".to_owned());
        }
        let decoded = image::open(&canonical)
            .map_err(|error| format!("Impossible de décoder l’image: {error}"))?
            .into_rgba8();
        let (width, height) = decoded.dimensions();
        let mut clipboard =
            Clipboard::new().map_err(|error| format!("Presse-papiers indisponible: {error}"))?;
        clipboard
            .set_image(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(decoded.into_raw()),
            })
            .map_err(|error| format!("Impossible de copier l’image: {error}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_imagyx(
    image_id: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if !state
        .database
        .image_exists(&image_id)
        .map_err(|error| error.to_string())?
    {
        return Err("Image inconnue".to_owned());
    }
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Fenêtre principale indisponible".to_owned())?;
    main.show().map_err(|error| error.to_string())?;
    let _ = main.unminimize();
    main.set_focus().map_err(|error| error.to_string())?;
    main.emit("open-image-requested", image_id)
        .map_err(|error| error.to_string())?;
    if let Some(spotlight) = app.get_webview_window("spotlight") {
        let _ = spotlight.emit("spotlight-will-hide", ());
        let _ = spotlight.hide();
    }
    Ok(())
}

fn managed_path(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Chemin inaccessible: {error}"))?;
    let folders = state
        .database
        .folders()
        .map_err(|error| error.to_string())?;
    let allowed = folders
        .iter()
        .any(|folder| canonical.starts_with(Path::new(&folder.path)));
    if !allowed {
        return Err("Ce chemin ne fait pas partie d’un dossier suivi".to_owned());
    }
    Ok(canonical)
}
