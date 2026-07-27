use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const SPOTLIGHT_WIDTH: f64 = 780.0;
const SPOTLIGHT_COMPACT_HEIGHT: f64 = 126.0;
const SPOTLIGHT_EXPANDED_HEIGHT: f64 = 580.0;

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
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "Fenêtre Spotlight indisponible".to_owned())?;
    let _ = window.emit("spotlight-will-hide", ());
    window.hide().map_err(|error| error.to_string())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
