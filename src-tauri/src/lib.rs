mod commands;
mod db;
mod indexer;
mod ml;
mod models;
mod onboarding;
mod paths;
mod preferences;
mod state;
mod system_stats;
mod thumbnails;
mod watcher;

use std::{path::PathBuf, sync::Arc};

use paths::AppPaths;
use preferences::ShortcutPreferences;
use state::AppState;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;
use thiserror::Error;
use watcher::FolderWatcher;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("pictures directory is unavailable")]
    PicturesDirectoryUnavailable,
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("model error: {0}")]
    Model(String),
    #[error("filesystem watcher error: {0}")]
    Watcher(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state() != ShortcutState::Pressed { return; }
                    let Some(window) = app.get_webview_window("spotlight") else { return; };
                    let visible = window.is_visible().unwrap_or(false);
                    if visible {
                        let _ = window.emit("spotlight-will-hide", ());
                        let _ = window.hide();
                    } else {
                        let _ = window.emit("spotlight-will-open", ());
                        if let Err(error) = commands::layout_spotlight_window(&window, false) {
                            eprintln!("Impossible de positionner Spotlight: {error}");
                        }
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("spotlight-opened", ());
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let paths = AppPaths::discover()?;
            let shortcut_preferences = ShortcutPreferences::load(paths.root.join("settings.json"));
            if let Err(error) = shortcut_preferences.register(app.handle()) {
                eprintln!("Impossible d’enregistrer le raccourci Spotlight: {error}");
            }

            let state = Arc::new(AppState::new(paths)?);
            let folders = state.database.folders()?;
            app.asset_protocol_scope().allow_directory(&state.paths.thumbnails, true)?;
            app.asset_protocol_scope().allow_directory(&state.paths.models, true)?;
            for folder in &folders { app.asset_protocol_scope().allow_directory(&folder.path, true)?; }
            let folder_watcher = FolderWatcher::start(app.handle().clone(), Arc::clone(&state), folders)?;
            app.manage(shortcut_preferences);
            app.manage(folder_watcher);
            app.manage(state);

            if let Some(main) = app.get_webview_window("main") {
                let window = main.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });
            }
            if let Some(spotlight) = app.get_webview_window("spotlight") {
                let window = spotlight.clone();
                spotlight.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.emit("spotlight-will-hide", ());
                        let _ = window.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_platform,
            commands::get_runtime_stats,
            commands::update_runtime_stats,
            commands::update_model_progress,
            commands::get_spotlight_shortcut,
            commands::set_spotlight_shortcut,
            commands::prepare_local_model,
            commands::reset_embeddings,
            commands::list_folders,
            commands::add_folder,
            commands::remove_folder,
            commands::index_folder,
            commands::pending_images,
            commands::prepare_ai_images,
            commands::save_embeddings,
            commands::explain_results,
            commands::top_image_tags,
            commands::get_thumbnail,
            commands::search_images,
            commands::open_in_file_manager,
            commands::copy_image_to_clipboard,
            commands::open_in_imagyx,
            onboarding::open_onboarding,
            commands::set_spotlight_expanded,
            commands::hide_spotlight,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Imagyx");
}
