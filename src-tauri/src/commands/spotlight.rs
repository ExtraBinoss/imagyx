use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow,
    WebviewWindowBuilder,
    webview::PageLoadEvent,
};

use crate::tracing;

const SPOTLIGHT_WIDTH: f64 = 780.0;
const SPOTLIGHT_COMPACT_HEIGHT: f64 = 126.0;
const SPOTLIGHT_EXPANDED_HEIGHT: f64 = 580.0;
static SPOTLIGHT_CREATING: AtomicBool = AtomicBool::new(false);
static SPOTLIGHT_READY: AtomicBool = AtomicBool::new(false);
static SPOTLIGHT_SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn toggle_spotlight(app: AppHandle) {
    request_spotlight(app, true);
}

pub(crate) fn show_spotlight(app: AppHandle) {
    request_spotlight(app, false);
}

pub(crate) fn prewarm_spotlight(app: AppHandle) {
    if app.get_webview_window("spotlight").is_some() {
        return;
    }
    SPOTLIGHT_READY.store(false, Ordering::Release);
    if SPOTLIGHT_CREATING.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawn_result = std::thread::Builder::new()
        .name("imagyx-spotlight-prewarm".into())
        .spawn(move || create_spotlight_window(&app));
    if let Err(error) = spawn_result {
        SPOTLIGHT_CREATING.store(false, Ordering::Release);
        tracing::event("spotlight.prewarm_thread.failed", error);
    }
}

fn request_spotlight(app: AppHandle, toggle_existing: bool) {
    if let Some(window) = app.get_webview_window("spotlight") {
        if !SPOTLIGHT_READY.load(Ordering::Acquire) {
            SPOTLIGHT_SHOW_REQUESTED.store(true, Ordering::Release);
            return;
        }
        if toggle_existing {
            toggle_existing_window(&window);
        } else {
            show_spotlight_window(&window);
        }
        return;
    }

    SPOTLIGHT_READY.store(false, Ordering::Release);
    SPOTLIGHT_SHOW_REQUESTED.store(true, Ordering::Release);
    if SPOTLIGHT_CREATING.swap(true, Ordering::AcqRel) {
        return;
    }

    let thread_app = app.clone();
    let spawn_result = std::thread::Builder::new()
        .name("imagyx-spotlight-window".into())
        .spawn(move || create_spotlight_window(&thread_app));
    if let Err(error) = spawn_result {
        SPOTLIGHT_CREATING.store(false, Ordering::Release);
        SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
        tracing::event("spotlight.create_thread.failed", error);
    }
}

fn create_spotlight_window(app: &AppHandle) {
    let _trace = tracing::span("spotlight.window.create");
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == "spotlight")
    else {
        SPOTLIGHT_CREATING.store(false, Ordering::Release);
        SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
        tracing::event("spotlight.config.missing", "label=spotlight");
        return;
    };

    let builder = match WebviewWindowBuilder::from_config(app, config) {
        Ok(builder) => builder,
        Err(error) => {
            SPOTLIGHT_CREATING.store(false, Ordering::Release);
            SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
            tracing::event("spotlight.builder.failed", error);
            return;
        }
    };

    let builder = builder.on_page_load(|window, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }
        tracing::event("spotlight.page.loaded", window.label());
        // The frontend confirms readiness once its event listeners are installed.
        // A fixed grace period races cold webview startup and can lose the open event.
    });

    match builder.build() {
        Ok(window) => attach_close_handler(&window),
        Err(error) => {
            SPOTLIGHT_CREATING.store(false, Ordering::Release);
            SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
            tracing::event("spotlight.build.failed", error);
        }
    }
}

#[tauri::command]
pub fn spotlight_frontend_ready(app: AppHandle) {
    let Some(window) = app.get_webview_window("spotlight") else {
        tracing::event("spotlight.frontend_ready.window_missing", "label=spotlight");
        return;
    };

    if SPOTLIGHT_READY.swap(true, Ordering::AcqRel) {
        return;
    }
    tracing::event("spotlight.frontend.ready", window.label());
    SPOTLIGHT_CREATING.store(false, Ordering::Release);
    if SPOTLIGHT_SHOW_REQUESTED.swap(false, Ordering::AcqRel) {
        show_spotlight_window(&window);
    }
}

fn attach_close_handler(window: &WebviewWindow) {
    let close_window = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
            let _ = close_window.emit("spotlight-will-hide", ());
            let _ = close_window.hide();
        }
    });
}

fn toggle_existing_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
        let _ = window.emit("spotlight-will-hide", ());
        let _ = window.hide();
    } else {
        show_spotlight_window(window);
    }
}

fn show_spotlight_window(window: &WebviewWindow) {
    let _trace = tracing::span("spotlight.window.show");
    let _ = window.emit("spotlight-will-open", ());
    if let Err(error) = layout_spotlight_window(window, false) {
        tracing::event("spotlight.layout.failed", error);
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("spotlight-opened", ());
}

pub(crate) fn layout_spotlight_window(
    window: &WebviewWindow,
    expanded: bool,
) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "Écran actif indisponible".to_owned())?;
    let scale = monitor.scale_factor();
    let logical_height = if expanded {
        SPOTLIGHT_EXPANDED_HEIGHT
    } else {
        SPOTLIGHT_COMPACT_HEIGHT
    };
    let width = (SPOTLIGHT_WIDTH * scale).round().max(1.0) as u32;
    let height = (logical_height * scale).round().max(1.0) as u32;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;

    let work_area = monitor.work_area();
    let left = i64::from(work_area.position.x);
    let top = i64::from(work_area.position.y);
    let available_width = i64::from(work_area.size.width);
    let available_height = i64::from(work_area.size.height);
    let x = left + ((available_width - i64::from(width)) / 2).max(0);
    let desired_y = top + (available_height as f64 * 0.25).round() as i64;
    let maximum_y = top + (available_height - i64::from(height)).max(0);
    let y = desired_y.min(maximum_y).max(top);
    window
        .set_position(PhysicalPosition::new(clamp_i32(x), clamp_i32(y)))
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_spotlight_expanded(expanded: bool, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "Fenêtre Spotlight indisponible".to_owned())?;
    layout_spotlight_window(&window, expanded)
}

#[tauri::command]
pub fn hide_spotlight(app: AppHandle) -> Result<(), String> {
    SPOTLIGHT_SHOW_REQUESTED.store(false, Ordering::Release);
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "Fenêtre Spotlight indisponible".to_owned())?;
    let _ = window.emit("spotlight-will-hide", ());
    window.hide().map_err(|error| error.to_string())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
