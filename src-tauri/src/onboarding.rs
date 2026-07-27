use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn open_onboarding(app: AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Fenêtre principale indisponible".to_owned())?;

    main.show().map_err(|error| error.to_string())?;
    let _ = main.unminimize();
    main.set_focus().map_err(|error| error.to_string())?;
    main.emit("open-onboarding-requested", ())
        .map_err(|error| error.to_string())?;

    if let Some(spotlight) = app.get_webview_window("spotlight") {
        let _ = spotlight.emit("spotlight-will-hide", ());
        let _ = spotlight.hide();
    }

    Ok(())
}
