use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};

use tauri::{AppHandle, Emitter};

use crate::{models::ModelDownloadProgress, state::AppState};

pub const MODEL_ID: &str = "active-transformersjs-model";

struct ModelSpec {
    name: &'static str,
    repository: &'static str,
    directory: &'static str,
    files: &'static [&'static str],
}

const S0_FILES: &[&str] = &[
    "config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "onnx/vision_model.onnx",
    "onnx/text_model.onnx",
];

const S2_FILES: &[&str] = &[
    "config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "onnx/s2/vision_model.onnx",
    "onnx/s2/text_model.onnx",
];

fn model_spec(key: &str) -> Result<ModelSpec, String> {
    match key {
        "mobileclip-s0" => Ok(ModelSpec {
            name: "MobileCLIP-S0",
            repository: "Xenova/mobileclip_s0",
            directory: "Xenova/mobileclip_s0",
            files: S0_FILES,
        }),
        "mobileclip2-s2" => Ok(ModelSpec {
            name: "MobileCLIP2-S2",
            repository: "plhery/mobileclip2-onnx",
            directory: "plhery/mobileclip2-onnx",
            files: S2_FILES,
        }),
        _ => Err(format!("Modèle inconnu: {key}")),
    }
}

pub fn prepare_local_model(
    app: &AppHandle,
    state: &Arc<AppState>,
    model_key: &str,
) -> Result<PathBuf, String> {
    let _guard = state.lock_model();
    let spec = model_spec(model_key)?;
    let model_root = state.paths.models.join(spec.directory);
    fs::create_dir_all(&model_root).map_err(|error| error.to_string())?;

    let missing = spec
        .files
        .iter()
        .copied()
        .filter(|relative| !model_root.join(relative).is_file())
        .collect::<Vec<_>>();

    if missing.is_empty() {
        let size = spec
            .files
            .iter()
            .filter_map(|relative| fs::metadata(model_root.join(relative)).ok())
            .map(|metadata| metadata.len())
            .sum();
        *state.model_progress.write() = ModelDownloadProgress {
            stage: "ready".into(),
            file_name: None,
            current_bytes: size,
            total_bytes: size,
            current_file: spec.files.len(),
            total_files: spec.files.len(),
            message: format!("{} est disponible hors connexion.", spec.name),
        };
        return Ok(model_root);
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("imagyx/{}", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|error| error.to_string())?;

    let mut sizes = Vec::with_capacity(missing.len());
    let mut total = 0_u64;
    for relative in &missing {
        let url = model_url(&spec, relative);
        let size = client
            .head(&url)
            .send()
            .ok()
            .and_then(|response| {
                response
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse().ok())
            })
            .unwrap_or(0);
        total = total.saturating_add(size);
        sizes.push(size);
    }

    publish(
        app,
        state,
        ModelDownloadProgress {
            stage: "downloading".into(),
            file_name: None,
            current_bytes: 0,
            total_bytes: total,
            current_file: 0,
            total_files: missing.len(),
            message: format!("Téléchargement de {}…", spec.name),
        },
    );

    let mut completed = 0_u64;
    for (index, (relative, expected)) in missing.iter().zip(sizes).enumerate() {
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
            .get(model_url(&spec, relative))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("Téléchargement de {relative}: {error}"))?;
        let response_size = response.content_length().unwrap_or(expected);
        if total == 0 {
            total = response_size;
        }
        let mut output = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 256 * 1024];
        let mut current = 0_u64;
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
            current = current.saturating_add(read as u64);
            publish(
                app,
                state,
                ModelDownloadProgress {
                    stage: "downloading".into(),
                    file_name: Some((*relative).into()),
                    current_bytes: completed.saturating_add(current),
                    total_bytes: total.max(completed + response_size),
                    current_file: index + 1,
                    total_files: missing.len(),
                    message: format!("{} · {}", spec.name, relative),
                },
            );
        }
        output.flush().map_err(|error| error.to_string())?;
        if response_size > 0 && current != response_size {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "Fichier incomplet pour {relative}: {current}/{response_size} octets"
            ));
        }
        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
        completed = completed.saturating_add(current);
    }

    publish(
        app,
        state,
        ModelDownloadProgress {
            stage: "ready".into(),
            file_name: None,
            current_bytes: completed,
            total_bytes: total.max(completed),
            current_file: missing.len(),
            total_files: missing.len(),
            message: format!("{} est disponible hors connexion.", spec.name),
        },
    );
    Ok(model_root)
}

fn model_url(spec: &ModelSpec, relative: &str) -> String {
    format!(
        "https://huggingface.co/{}/resolve/main/{relative}?download=true",
        spec.repository
    )
}

fn publish(app: &AppHandle, state: &Arc<AppState>, progress: ModelDownloadProgress) {
    *state.model_progress.write() = progress.clone();
    let _ = app.emit("model-download-progress", progress);
}
