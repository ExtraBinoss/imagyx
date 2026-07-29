mod commands;
mod database;
mod fuzzy;
mod indexer;
mod ml;
mod models;
mod onboarding;
mod paths;
mod preferences;
mod state;
mod system_stats;
mod thumbnails;
mod tracing;
mod tray;
mod vector_store;
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
    tracing::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        commands::spotlight::toggle_spotlight(app.clone());
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let _setup_trace = tracing::span("startup.setup");
            let paths = {
                let _trace = tracing::span("startup.paths");
                AppPaths::discover()?
            };
            let shortcut_preferences = {
                let _trace = tracing::span("startup.shortcut.load");
                ShortcutPreferences::load(paths.root.join("settings.json"))
            };
            {
                let _trace = tracing::span("startup.shortcut.register");
                if let Err(error) = shortcut_preferences.register(app.handle()) {
                    tracing::event("shortcut.register.failed", error);
                }
            }

            let state = {
                let _trace = tracing::span("startup.state");
                Arc::new(AppState::new(paths)?)
            };
            let tray_snapshot = {
                let library_folders = state.database.folders()?;
                tray::LibrarySnapshot {
                    folder_count: library_folders.len(),
                    image_count: library_folders
                        .into_iter()
                        .filter_map(|folder| usize::try_from(folder.image_count).ok())
                        .sum(),
                }
            };
            let should_open_main = tray_snapshot.folder_count == 0;
            let folders = {
                let _trace = tracing::span("startup.folders");
                state.database.folders_for_watching()?
            };
            {
                let _trace = tracing::span("startup.asset_scopes");
                app.asset_protocol_scope()
                    .allow_directory(&state.paths.thumbnails, true)?;
                app.asset_protocol_scope()
                    .allow_directory(&state.paths.models, true)?;
                for folder in &folders {
                    app.asset_protocol_scope()
                        .allow_directory(&folder.path, true)?;
                }
            }
            let folder_watcher = {
                let _trace = tracing::span("startup.watcher");
                FolderWatcher::start(app.handle().clone(), Arc::clone(&state), folders)?
            };

            let vector_state = Arc::clone(&state);
            let vector_app = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let _trace = tracing::span("startup.load_vectors");
                match vector_state.load_vectors() {
                    Ok(()) => {
                        let _ = vector_app.emit("vectors-ready", ());
                    }
                    Err(error) => tracing::event("startup.load_vectors.failed", error),
                }
            });

            app.manage(shortcut_preferences);
            app.manage(folder_watcher);
            app.manage(Arc::clone(&state));
            tray::setup(app.handle(), tray_snapshot)?;

            #[cfg(all(target_os = "macos", not(debug_assertions)))]
            {
                app.handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory)?;
                app.handle().set_dock_visibility(false)?;
            }

            if let Some(main) = app.get_webview_window("main") {
                let window = main.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });

                if cfg!(debug_assertions) || should_open_main {
                    let _ = main.show();
                    let _ = main.set_focus();
                }
            }
            commands::spotlight::prewarm_spotlight(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::app::get_platform,
            commands::app::get_index_coverage,
            commands::app::get_runtime_stats,
            commands::app::update_runtime_stats,
            commands::app::update_model_progress,
            commands::app::get_spotlight_shortcut,
            commands::app::set_spotlight_shortcut,
            commands::app::prepare_local_model,
            commands::app::reset_embeddings,
            commands::folders::list_folders,
            commands::folders::add_folder,
            commands::folders::remove_folder,
            commands::folders::index_folder,
            commands::folders::pending_images,
            commands::semantic::prepare_ai_images,
            commands::semantic::prepare_visual_query_image,
            commands::semantic::get_image_embedding,
            commands::semantic::save_embeddings,
            commands::semantic::explain_results,
            commands::semantic::top_image_tags,
            commands::search::get_thumbnail,
            commands::search::search_images,
            commands::search::search_image_page,
            commands::spotlight::spotlight_frontend_ready,
            commands::files::open_in_file_manager,
            commands::files::copy_image_to_clipboard,
            commands::files::convert_image,
            commands::files::open_in_imagyx,
            onboarding::open_onboarding,
            commands::spotlight::set_spotlight_expanded,
            commands::spotlight::hide_spotlight,
            tray::set_tray_paused,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Imagyx");
}
