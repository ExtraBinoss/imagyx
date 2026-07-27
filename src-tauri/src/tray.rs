use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use tauri::{
    AppHandle, Emitter, Manager, Wry,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{
    commands::spotlight,
    models::{IndexProgress, RuntimeStats},
    state::AppState,
    tracing,
};

const TRAY_ID: &str = "imagyx-background";
const ID_ADD_FOLDER: &str = "add-folder";
const ID_OPEN_SPOTLIGHT: &str = "open-spotlight";
const ID_OPEN_IMAGYX: &str = "open-imagyx";
const ID_PAUSE_INDEXING: &str = "pause-indexing";
const ID_QUIT: &str = "quit-imagyx";

const IDLE_ICON: Image<'static> = tauri::include_image!("./icons/Square310x310Logo.png");
const BUSY_ICON: Image<'static> = tauri::include_image!("./icons/imagyx-bigger-searching.ico");

#[derive(Debug, Clone, Copy, Default)]
pub struct LibrarySnapshot {
    pub folder_count: usize,
    pub image_count: usize,
}

pub struct TrayController {
    status_item: MenuItem<Wry>,
    pause_item: MenuItem<Wry>,
    paused: AtomicBool,
    busy: AtomicBool,
    current: AtomicUsize,
    total: AtomicUsize,
}

pub fn setup(app: &AppHandle, snapshot: LibrarySnapshot) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(
        app,
        "index-status",
        ready_label(snapshot),
        false,
        None::<&str>,
    )?;
    let add_folder = MenuItem::with_id(
        app,
        ID_ADD_FOLDER,
        "Add folder…",
        true,
        None::<&str>,
    )?;
    let open_spotlight = MenuItem::with_id(
        app,
        ID_OPEN_SPOTLIGHT,
        "Open Spotlight",
        true,
        None::<&str>,
    )?;
    let open_imagyx = MenuItem::with_id(
        app,
        ID_OPEN_IMAGYX,
        "Open Imagyx",
        true,
        None::<&str>,
    )?;
    let pause_item = MenuItem::with_id(
        app,
        ID_PAUSE_INDEXING,
        "Pause indexing",
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Quit Imagyx", true, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &separator_one,
            &add_folder,
            &open_spotlight,
            &open_imagyx,
            &pause_item,
            &separator_two,
            &quit,
        ],
    )?;

    app.manage(TrayController {
        status_item,
        pause_item,
        paused: AtomicBool::new(false),
        busy: AtomicBool::new(false),
        current: AtomicUsize::new(snapshot.image_count),
        total: AtomicUsize::new(snapshot.image_count),
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(IDLE_ICON)
        .tooltip(ready_tooltip(snapshot))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            ID_ADD_FOLDER => emit_add_folder(app),
            ID_OPEN_SPOTLIGHT => spotlight::show_spotlight(app.clone()),
            ID_OPEN_IMAGYX => show_main_window(app),
            ID_PAUSE_INDEXING => toggle_pause(app),
            ID_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                spotlight::show_spotlight(tray.app_handle().clone());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_index_progress(app: &AppHandle, progress: &IndexProgress) {
    match progress.stage.as_str() {
        "complete" => set_ready(app, library_snapshot(app)),
        "error" => set_error(app, &progress.folder_name),
        "discovering" => set_indexing(app, progress.current, progress.total, "Scanning folder…"),
        "metadata" => set_indexing(
            app,
            progress.current,
            progress.total,
            &format!("Reading metadata · {} of {}", progress.current, progress.total),
        ),
        "queued" => set_indexing(
            app,
            progress.current,
            progress.total,
            "Waiting for AI indexing",
        ),
        "embedding" | "saving" => {
            set_indexing(app, progress.current, progress.total, "AI indexing");
        }
        _ => {}
    }
}

pub fn update_runtime_stats(app: &AppHandle, stats: &RuntimeStats) {
    match stats.stage.as_str() {
        "indexing" | "decoding" | "inference" | "saving" => {
            set_indexing(
                app,
                stats.current,
                stats.total,
                &format!("AI indexing · {} of {}", stats.current, stats.total),
            );
        }
        "paused" => set_paused(app, true),
        "ready" if stats.total > 0 && stats.current >= stats.total => {
            set_ready(app, library_snapshot(app));
        }
        _ => {}
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_tray_paused(paused: bool, app: AppHandle) {
    set_paused(&app, paused);
}

fn emit_add_folder(app: &AppHandle) {
    if let Err(error) = app.emit("add-folder-requested", ()) {
        tracing::event("tray.add_folder.failed", error);
    }
}

fn toggle_pause(app: &AppHandle) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    let paused = !controller.paused.load(Ordering::Acquire);
    drop(controller);
    set_paused(app, paused);
    let event = if paused {
        "pause-indexing-requested"
    } else {
        "resume-indexing-requested"
    };
    if let Err(error) = app.emit(event, ()) {
        tracing::event("tray.indexing_control.failed", error);
    }
}

fn set_indexing(app: &AppHandle, current: usize, total: usize, message: &str) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    controller.current.store(current, Ordering::Release);
    controller.total.store(total, Ordering::Release);
    controller.paused.store(false, Ordering::Release);
    let _ = controller.pause_item.set_text("Pause indexing");
    let _ = controller.pause_item.set_enabled(true);

    let label = if total > 0 {
        format!("Indexing · {current}/{total}")
    } else {
        "Preparing indexing…".to_owned()
    };
    let tooltip = if total > 0 {
        format!("Imagyx — {message} ({current}/{total})")
    } else {
        format!("Imagyx — {message}")
    };
    let _ = controller.status_item.set_text(label);
    update_tray_visual(app, &controller, true, tooltip);
}

fn set_paused(app: &AppHandle, paused: bool) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    controller.paused.store(paused, Ordering::Release);
    let current = controller.current.load(Ordering::Acquire);
    let total = controller.total.load(Ordering::Acquire);

    if paused {
        let label = if total > 0 {
            format!("Paused · {current}/{total}")
        } else {
            "Indexing paused".to_owned()
        };
        let _ = controller.status_item.set_text(&label);
        let _ = controller.pause_item.set_text("Resume indexing");
        let _ = controller.pause_item.set_enabled(true);
        update_tray_visual(app, &controller, false, format!("Imagyx — {label}"));
        return;
    }

    let _ = controller.pause_item.set_text("Pause indexing");
    drop(controller);
    if total > current {
        set_indexing(app, current, total, "Resuming indexing");
    } else {
        set_ready(app, library_snapshot(app));
    }
}

fn set_ready(app: &AppHandle, snapshot: LibrarySnapshot) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    controller
        .current
        .store(snapshot.image_count, Ordering::Release);
    controller
        .total
        .store(snapshot.image_count, Ordering::Release);
    controller.paused.store(false, Ordering::Release);
    let _ = controller.status_item.set_text(ready_label(snapshot));
    let _ = controller.pause_item.set_text("Pause indexing");
    let _ = controller.pause_item.set_enabled(false);
    update_tray_visual(app, &controller, false, ready_tooltip(snapshot));
}

fn set_error(app: &AppHandle, folder_name: &str) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    let label = format!("Indexing error · {folder_name}");
    let _ = controller.status_item.set_text(&label);
    let _ = controller.pause_item.set_enabled(false);
    update_tray_visual(app, &controller, false, format!("Imagyx — {label}"));
}

fn update_tray_visual(
    app: &AppHandle,
    controller: &TrayController,
    busy: bool,
    tooltip: String,
) {
    let was_busy = controller.busy.swap(busy, Ordering::AcqRel);
    if let Some(icon) = app.tray_by_id(TRAY_ID) {
        if was_busy != busy {
            let image = if busy {
                BUSY_ICON.clone()
            } else {
                IDLE_ICON.clone()
            };
            if let Err(error) = icon.set_icon(Some(image)) {
                tracing::event("tray.icon.update.failed", error);
            }
        }
        if let Err(error) = icon.set_tooltip(Some(tooltip)) {
            tracing::event("tray.tooltip.update.failed", error);
        }
    }
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn library_snapshot(app: &AppHandle) -> LibrarySnapshot {
    app.try_state::<Arc<AppState>>()
        .and_then(|state| state.database.folders().ok())
        .map(|folders| LibrarySnapshot {
            folder_count: folders.len(),
            image_count: folders
                .into_iter()
                .filter_map(|folder| usize::try_from(folder.image_count).ok())
                .sum(),
        })
        .unwrap_or_default()
}

fn ready_label(snapshot: LibrarySnapshot) -> String {
    if snapshot.folder_count == 0 {
        "No folders added".to_owned()
    } else {
        "Imagyx — OK".to_owned()
    }
}

fn ready_tooltip(snapshot: LibrarySnapshot) -> String {
    if snapshot.folder_count == 0 {
        return "Imagyx — Add a folder to get started".to_owned();
    }
    match snapshot.image_count {
        0 => "Imagyx — Folder ready, no images indexed".to_owned(),
        1 => "Imagyx — 1 image indexed".to_owned(),
        count => format!("Imagyx — {count} images indexed"),
    }
}
