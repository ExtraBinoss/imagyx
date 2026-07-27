use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const DEFAULT_SPOTLIGHT_SHORTCUT: &str = "Control+Numpad9";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreferencesFile {
    spotlight_shortcut: String,
}

pub struct ShortcutPreferences {
    path: PathBuf,
    current: Mutex<(String, Shortcut)>,
}

impl ShortcutPreferences {
    pub fn load(path: PathBuf) -> Self {
        let requested = read_preferences(&path)
            .map(|preferences| preferences.spotlight_shortcut)
            .unwrap_or_else(|| DEFAULT_SPOTLIGHT_SHORTCUT.to_owned());
        let shortcut = Shortcut::from_str(&requested)
            .or_else(|_| Shortcut::from_str(DEFAULT_SPOTLIGHT_SHORTCUT))
            .expect("default spotlight shortcut must be valid");
        let normalized = shortcut.clone().into_string();
        Self {
            path,
            current: Mutex::new((normalized, shortcut)),
        }
    }

    pub fn register(&self, app: &AppHandle) -> Result<(), String> {
        let current = self.current.lock();
        app.global_shortcut()
            .register(current.1.clone())
            .map_err(|error| error.to_string())
    }

    pub fn value(&self) -> String {
        self.current.lock().0.clone()
    }

    pub fn update(&self, app: &AppHandle, requested: &str) -> Result<String, String> {
        let parsed = Shortcut::from_str(requested)
            .map_err(|error| format!("Raccourci invalide: {error}"))?;
        let normalized = parsed.clone().into_string();
        let mut current = self.current.lock();
        if current.0 == normalized {
            return Ok(normalized);
        }

        let previous_shortcut = current.1.clone();
        app.global_shortcut()
            .unregister(previous_shortcut.clone())
            .map_err(|error| format!("Impossible de libérer l’ancien raccourci: {error}"))?;

        if let Err(error) = app.global_shortcut().register(parsed.clone()) {
            let _ = app.global_shortcut().register(previous_shortcut);
            return Err(format!("Ce raccourci n’est pas disponible: {error}"));
        }

        if let Err(error) = write_preferences(
            &self.path,
            &PreferencesFile {
                spotlight_shortcut: normalized.clone(),
            },
        ) {
            let _ = app.global_shortcut().unregister(parsed);
            let _ = app.global_shortcut().register(previous_shortcut);
            return Err(format!("Impossible d’enregistrer le raccourci: {error}"));
        }

        current.0 = normalized.clone();
        current.1 = parsed;
        Ok(normalized)
    }
}

fn read_preferences(path: &Path) -> Option<PreferencesFile> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_preferences(path: &Path, preferences: &PreferencesFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(preferences).map_err(|error| error.to_string())?;
    fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}
