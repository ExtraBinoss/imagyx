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
const ID_OPEN_SPOTLIGHT: &str = "open-spotlight";
const ID_OPEN_IMAGYX: &str = "open-imagyx";
const ID_PAUSE_INDEXING: &str = "pause-indexing";
const ID_QUIT: &str = "quit-imagyx";

const IDLE_ICON: Image<'static> = tauri::include_image!("./icons/imagyx.ico");
const BUSY_ICON: Image<'static> = tauri::include_image!("./icons/imagyx-searching.ico");

pub struct TrayController {
    status_item: MenuItem<Wry>,
    pause_item: MenuItem<Wry>,
    paused: AtomicBool,
    busy: AtomicBool,
    current: AtomicUsize,
    total: AtomicUsize,
}

pub fn setup(app: &AppHandle, indexed_images: usize) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(
        app,
        "index-status",
        ready_label(indexed_images),
        false,
        None::<&str>,
    )?;
    let open_spotlight = MenuItem::with_id(
        app,
        ID_OPEN_SPOTLIGHT,
        "Ouvrir Spotlight",
        true,
        None::<&str>,
    )?;
    let open_imagyx = MenuItem::with_id(
        app,
        ID_OPEN_IMAGYX,
        "Ouvrir Imagyx",
        true,
        None::<&str>,
    )?;
    let pause_item = MenuItem::with_id(
        app,
        ID_PAUSE_INDEXING,
        "Mettre l’indexation en pause",
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Quitter Imagyx", true, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &separator_one,
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
        current: AtomicUsize::new(indexed_images),
        total: AtomicUsize::new(indexed_images),
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(IDLE_ICON)
        .tooltip(ready_tooltip(indexed_images))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
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
        "complete" => set_ready(app, indexed_image_count(app)),
        "error" => set_error(app, &progress.folder_name),
        "queued" | "discovering" | "metadata" | "embedding" | "saving" => {
            set_indexing(app, progress.current, progress.total, &progress.message);
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
                &format!("Analyse IA · {} sur {}", stats.current, stats.total),
            );
        }
        "paused" => set_paused(app, true),
        "ready" if stats.total > 0 && stats.current >= stats.total => {
            set_ready(app, indexed_image_count(app));
        }
        _ => {}
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_tray_paused(paused: bool, app: AppHandle) {
    set_paused(&app, paused);
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
    let _ = controller.pause_item.set_text("Mettre l’indexation en pause");
    let _ = controller.pause_item.set_enabled(true);

    let label = if total > 0 {
        format!("Indexation · {current}/{total}")
    } else {
        "Préparation de l’indexation…".to_owned()
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
            format!("En pause · {current}/{total}")
        } else {
            "Indexation en pause".to_owned()
        };
        let _ = controller.status_item.set_text(&label);
        let _ = controller.pause_item.set_text("Reprendre l’indexation");
        let _ = controller.pause_item.set_enabled(true);
        update_tray_visual(app, &controller, false, format!("Imagyx — {label}"));
    } else {
        let _ = controller.pause_item.set_text("Mettre l’indexation en pause");
        if total > current {
            set_indexing(app, current, total, "Reprise de l’indexation");
        } else {
            set_ready(app, indexed_image_count(app));
        }
    }
}

fn set_ready(app: &AppHandle, indexed_images: usize) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    controller.current.store(indexed_images, Ordering::Release);
    controller.total.store(indexed_images, Ordering::Release);
    controller.paused.store(false, Ordering::Release);
    let _ = controller.status_item.set_text(ready_label(indexed_images));
    let _ = controller.pause_item.set_text("Mettre l’indexation en pause");
    let _ = controller.pause_item.set_enabled(false);
    update_tray_visual(app, &controller, false, ready_tooltip(indexed_images));
}

fn set_error(app: &AppHandle, folder_name: &str) {
    let Some(controller) = app.try_state::<TrayController>() else {
        return;
    };
    let label = format!("Erreur d’indexation · {folder_name}");
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
            let image = if busy { BUSY_ICON.clone() } else { IDLE_ICON.clone() };
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

fn indexed_image_count(app: &AppHandle) -> usize {
    app.try_state::<Arc<AppState>>()
        .and_then(|state| state.database.folders_for_watching().ok())
        .map(|folders| folders.into_iter().map(|folder| folder.image_count).sum())
        .unwrap_or_default()
}

fn ready_label(indexed_images: usize) -> String {
    match indexed_images {
        0 => "Aucun dossier à analyser".to_owned(),
        1 => "À jour · 1 image indexée".to_owned(),
        count => format!("À jour · {count} images indexées"),
    }
}

fn ready_tooltip(indexed_images: usize) -> String {
    match indexed_images {
        0 => "Imagyx — ajoute un dossier pour commencer".to_owned(),
        1 => "Imagyx — 1 image indexée".to_owned(),
        count => format!("Imagyx — {count} images indexées"),
    }
}
