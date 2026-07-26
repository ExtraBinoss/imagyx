use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use tauri::{AppHandle, Emitter};

use crate::{
    models::ModelDownloadProgress,
    state::AppState,
};

pub const MODEL_ID: &str = "mobileclip-s0-transformersjs";
pub const MODEL_NAME: &str = "MobileCLIP-S0";
pub const MODEL_REPOSITORY: &str = "Xenova/mobileclip_s0";

const MODEL_FILES: &[&str] = &[
    "config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "onnx/vision_model.onnx",
    "onnx/text_model.onnx",
];

pub fn local_model_root(models_dir: &Path) -> PathBuf {
    models_dir.join("Xenova").join("mobileclip_s0")
}

pub fn prepare_local_model(
    app: &AppHandle,
    state: &Arc<AppState>,
) -> Result<PathBuf, String> {
    let model_root = local_model_root(&state.paths.models);
    fs::create_dir_all(&model_root).map_err(|error| error.to_string())?;

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("imagyx/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?;

    let missing = MODEL_FILES
        .iter()
        .copied()
        .filter(|relative| !model_root.join(relative).is_file())
        .collect::<Vec<_>>();

    if missing.is_empty() {
        publish(
            app,
            state,
            ModelDownloadProgress {
                stage: "ready".to_owned(),
                file_name: None,
                current_bytes: 0,
                total_bytes: 0,
                current_file: MODEL_FILES.len(),
                total_files: MODEL_FILES.len(),
                message: format!("{MODEL_NAME} est disponible hors connexion."),
            },
        );
        return Ok(model_root);
    }

    let mut known_total = 0_u64;
    let mut file_sizes = Vec::with_capacity(missing.len());
    for relative in &missing {
        let url = model_url(relative);
        let size = client
            .head(&url)
            .send()
            .ok()
            .and_then(|response| response.content_length())
            .unwrap_or(0);
        known_total = known_total.saturating_add(size);
        file_sizes.push(size);
    }

    let mut completed = 0_u64;
    for (index, (relative, expected_size)) in missing.iter().zip(file_sizes).enumerate() {
        let destination = model_root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let temporary = destination.with_extension(format!(
            "{}.download",
            destination
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("file")
        ));

        let mut response = client
            .get(model_url(relative))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("Téléchargement de {relative}: {error}"))?;
        let response_size = response.content_length().unwrap_or(expected_size);
        let mut output = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 256 * 1024];
        let mut current_file_bytes = 0_u64;

        loop {
            let read = response.read(&mut buffer).map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
            current_file_bytes = current_file_bytes.saturating_add(read as u64);
            publish(
                app,
                state,
                ModelDownloadProgress {
                    stage: "downloading".to_owned(),
                    file_name: Some((*relative).to_owned()),
                    current_bytes: completed.saturating_add(current_file_bytes),
                    total_bytes: known_total,
                    current_file: index + 1,
                    total_files: missing.len(),
                    message: format!("Téléchargement · {relative}"),
                },
            );
        }
        output.flush().map_err(|error| error.to_string())?;

        if response_size > 0 && current_file_bytes != response_size {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "Fichier incomplet pour {relative}: {current_file_bytes}/{response_size} octets"
            ));
        }
        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
        completed = completed.saturating_add(current_file_bytes);
    }

    publish(
        app,
        state,
        ModelDownloadProgress {
            stage: "ready".to_owned(),
            file_name: None,
            current_bytes: completed,
            total_bytes: known_total.max(completed),
            current_file: missing.len(),
            total_files: missing.len(),
            message: format!("{MODEL_NAME} est disponible hors connexion."),
        },
    );
    Ok(model_root)
}

fn model_url(relative: &str) -> String {
    format!(
        "https://huggingface.co/{MODEL_REPOSITORY}/resolve/main/{relative}?download=true"
    )
}

fn publish(app: &AppHandle, state: &Arc<AppState>, progress: ModelDownloadProgress) {
    *state.model_progress.write() = progress.clone();
    let _ = app.emit("model-download-progress", progress);
}
