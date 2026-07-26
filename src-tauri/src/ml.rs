use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use hf_hub::api::{Progress, sync::ApiBuilder};
use image::{DynamicImage, imageops::FilterType};
use ndarray::{Array2, Array4, Axis};
use ort::{
    ep::ExecutionProviderDispatch,
    session::{Session, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use parking_lot::RwLock;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokenizers::Tokenizer;

use crate::{
    AppError,
    models::{ModelDownloadProgress, ModelStatus, RuntimeStats},
};

pub const MODEL_ID: &str = "mobileclip2-s0";
pub const MODEL_NAME: &str = "MobileCLIP2-S0";
const MODEL_REPOSITORY: &str = "plhery/mobileclip2-onnx";
const IMAGE_EDGE: usize = 256;
const TEXT_CONTEXT: usize = 77;

#[derive(Clone, Copy)]
struct ModelFile {
    remote: &'static str,
    local: &'static str,
    label: &'static str,
    sha256: Option<&'static str>,
}

const MODEL_FILES: [ModelFile; 3] = [
    ModelFile {
        remote: "onnx/s0/vision_model.onnx",
        local: "mobileclip2-s0-image.onnx",
        label: "Encodeur visuel MobileCLIP2-S0",
        sha256: Some("13d20ebfa8a8f63890eb2727fe4dc63009ff970f43e0f7d9d2ed999659f70c8a"),
    },
    ModelFile {
        remote: "onnx/s0/text_model.onnx",
        local: "mobileclip2-s0-text.onnx",
        label: "Encodeur texte MobileCLIP2-S0",
        sha256: Some("df590d47744f2ee9f3ccb67c4414d17419568c05bca0c4d166f2faeedf8b92f3"),
    },
    ModelFile {
        remote: "tokenizer.json",
        local: "mobileclip2-tokenizer.json",
        label: "Tokenizer CLIP",
        sha256: None,
    },
];

#[derive(Debug)]
pub struct EmbeddingBatch {
    pub vectors: Vec<Vec<f32>>,
    pub decode_ms: u64,
    pub inference_ms: u64,
}

pub struct MlRuntime {
    cache_dir: PathBuf,
    image_session: Option<Session>,
    text_session: Option<Session>,
    tokenizer: Option<Tokenizer>,
    backend_requested: String,
    backend_effective: String,
    acceleration_active: bool,
    acceleration_label: String,
    batch_size: usize,
    last_error: Option<String>,
    fallback_reason: Option<String>,
}

impl std::fmt::Debug for MlRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MlRuntime")
            .field("cache_dir", &self.cache_dir)
            .field("ready", &self.is_ready())
            .field("backend_requested", &self.backend_requested)
            .field("backend_effective", &self.backend_effective)
            .field("batch_size", &self.batch_size)
            .field("last_error", &self.last_error)
            .field("fallback_reason", &self.fallback_reason)
            .finish()
    }
}

impl MlRuntime {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            image_session: None,
            text_session: None,
            tokenizer: None,
            backend_requested: requested_backend_label().to_owned(),
            backend_effective: "En attente".to_owned(),
            acceleration_active: false,
            acceleration_label: "Non initialisée".to_owned(),
            batch_size: cpu_batch_size(),
            last_error: None,
            fallback_reason: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.image_session.is_some() && self.text_session.is_some() && self.tokenizer.is_some()
    }

    pub fn backend_requested(&self) -> &str {
        &self.backend_requested
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn status(&self) -> ModelStatus {
        ModelStatus {
            ready: self.is_ready(),
            backend: self.backend_effective.clone(),
            acceleration_active: self.acceleration_active,
            acceleration_label: self.acceleration_label.clone(),
            fallback_reason: self.fallback_reason.clone(),
        }
    }

    pub fn prepare(
        &mut self,
        app: &AppHandle,
        progress_state: &RwLock<ModelDownloadProgress>,
        runtime_stats: &RwLock<RuntimeStats>,
    ) -> Result<(), AppError> {
        if self.is_ready() {
            publish_ready(app, progress_state, runtime_stats, self);
            return Ok(());
        }

        publish_progress(
            app,
            progress_state,
            ModelDownloadProgress {
                stage: "checking".to_owned(),
                file_name: None,
                current_bytes: 0,
                total_bytes: 0,
                current_file: 0,
                total_files: MODEL_FILES.len(),
                message: "Vérification de MobileCLIP2-S0…".to_owned(),
            },
        );
        {
            let mut stats = runtime_stats.write();
            stats.stage = "checking".to_owned();
            stats.model_name = MODEL_NAME.to_owned();
            stats.backend_requested = self.backend_requested.clone();
        }

        if let Err(error) = self.download_missing_files(app, progress_state) {
            self.last_error = Some(error.to_string());
            publish_error(app, progress_state, runtime_stats, self, &error.to_string());
            return Err(error);
        }

        publish_progress(
            app,
            progress_state,
            ModelDownloadProgress {
                stage: "loading".to_owned(),
                file_name: None,
                current_bytes: 0,
                total_bytes: 0,
                current_file: MODEL_FILES.len(),
                total_files: MODEL_FILES.len(),
                message: format!(
                    "Initialisation de MobileCLIP2-S0 avec {}…",
                    self.backend_requested
                ),
            },
        );
        runtime_stats.write().stage = "loading".to_owned();

        match self.initialize_models() {
            Ok(()) => {
                publish_ready(app, progress_state, runtime_stats, self);
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                publish_error(app, progress_state, runtime_stats, self, &error.to_string());
                Err(error)
            }
        }
    }

    pub fn ensure_ready(&mut self) -> Result<(), AppError> {
        if self.is_ready() {
            return Ok(());
        }
        if !self.cache_has_models() {
            return Err(AppError::Model(self.last_error.clone().unwrap_or_else(|| {
                "MobileCLIP2-S0 n’est pas encore prêt".to_owned()
            })));
        }
        self.initialize_models()
    }

    pub fn embed_images(&mut self, paths: &[PathBuf]) -> Result<EmbeddingBatch, AppError> {
        self.ensure_ready()?;
        let decode_started = Instant::now();
        let decoded = paths
            .par_iter()
            .map(|path| decode_mobileclip_image(path))
            .collect::<Result<Vec<_>, _>>()?;
        let decode_ms = millis(decode_started.elapsed());

        let flat = decoded.into_iter().flatten().collect::<Vec<_>>();
        let input = Array4::from_shape_vec((paths.len(), 3, IMAGE_EDGE, IMAGE_EDGE), flat)
            .map_err(|error| AppError::Model(error.to_string()))?;
        let tensor = TensorRef::from_array_view(&input)
            .map_err(|error| AppError::Model(error.to_string()))?;

        let inference_started = Instant::now();
        let session = self
            .image_session
            .as_mut()
            .ok_or_else(|| AppError::Model("encodeur image indisponible".into()))?;
        let outputs = session
            .run(ort::inputs!["pixel_values" => tensor])
            .map_err(|error| AppError::Model(error.to_string()))?;
        let output = outputs["image_embeds"]
            .try_extract_array::<f32>()
            .map_err(|error| AppError::Model(error.to_string()))?;
        let vectors = output
            .axis_iter(Axis(0))
            .map(|row| normalize(row.iter().copied().collect()))
            .collect();
        let inference_ms = millis(inference_started.elapsed());

        Ok(EmbeddingBatch {
            vectors,
            decode_ms,
            inference_ms,
        })
    }

    pub fn embed_text(&mut self, query: &str) -> Result<Vec<f32>, AppError> {
        self.ensure_ready()?;
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| AppError::Model("tokenizer indisponible".into()))?;
        let encoding = tokenizer
            .encode(query, true)
            .map_err(|error| AppError::Model(error.to_string()))?;
        let mut ids = encoding
            .get_ids()
            .iter()
            .take(TEXT_CONTEXT)
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        ids.resize(TEXT_CONTEXT, 0);
        let input = Array2::from_shape_vec((1, TEXT_CONTEXT), ids)
            .map_err(|error| AppError::Model(error.to_string()))?;
        let tensor = TensorRef::from_array_view(&input)
            .map_err(|error| AppError::Model(error.to_string()))?;
        let session = self
            .text_session
            .as_mut()
            .ok_or_else(|| AppError::Model("encodeur texte indisponible".into()))?;
        let outputs = session
            .run(ort::inputs!["input_ids" => tensor])
            .map_err(|error| AppError::Model(error.to_string()))?;
        let output = outputs["text_embeds"]
            .try_extract_array::<f32>()
            .map_err(|error| AppError::Model(error.to_string()))?;
        let first = output
            .axis_iter(Axis(0))
            .next()
            .ok_or_else(|| AppError::Model("embedding texte vide".into()))?;
        Ok(normalize(first.iter().copied().collect()))
    }

    pub fn cache_has_models(&self) -> bool {
        MODEL_FILES
            .iter()
            .all(|file| self.file_is_verified(*file).unwrap_or(false))
    }

    fn file_is_verified(&self, file: ModelFile) -> Result<bool, AppError> {
        let path = self.cache_dir.join(file.local);
        if !path.is_file() {
            return Ok(false);
        }
        let Some(expected) = file.sha256 else {
            return Ok(true);
        };
        let marker = verified_marker(&path);
        if fs::read_to_string(&marker).ok().as_deref() == Some(expected) {
            return Ok(true);
        }
        if !verify_sha256(&path, expected)? {
            return Ok(false);
        }
        fs::write(marker, expected)?;
        Ok(true)
    }

    fn initialize_models(&mut self) -> Result<(), AppError> {
        let image_path = self.cache_dir.join(MODEL_FILES[0].local);
        let text_path = self.cache_dir.join(MODEL_FILES[1].local);
        let tokenizer_path = self.cache_dir.join(MODEL_FILES[2].local);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|error| AppError::Model(format!("tokenizer MobileCLIP2-S0: {error}")))?;

        if let Some(acceleration) = acceleration_profile() {
            let accelerated = build_session(&image_path, acceleration.provider(), true, 1)
                .and_then(|image_session| {
                    build_session(&text_path, acceleration.provider(), true, 1)
                        .map(|text_session| (image_session, text_session))
                });
            match accelerated {
                Ok((image_session, text_session)) => {
                    self.image_session = Some(image_session);
                    self.text_session = Some(text_session);
                    self.tokenizer = Some(tokenizer);
                    self.backend_effective = acceleration.backend.to_owned();
                    self.acceleration_active = true;
                    self.acceleration_label = acceleration.label.to_owned();
                    self.batch_size = 48;
                    self.fallback_reason = None;
                    self.last_error = None;
                    return Ok(());
                }
                Err(error) => {
                    let reason = format!(
                        "{} indisponible ou modèle non entièrement pris en charge: {error}",
                        acceleration.backend
                    );
                    eprintln!("Imagyx hardware acceleration fallback: {reason}");
                    self.fallback_reason = Some(reason);
                }
            }
        }

        let threads = num_cpus::get().max(1);
        self.image_session = Some(build_session(&image_path, None, false, threads)?);
        self.text_session = Some(build_session(&text_path, None, false, threads)?);
        self.tokenizer = Some(tokenizer);
        self.backend_effective = "ONNX Runtime CPU".to_owned();
        self.acceleration_active = false;
        self.acceleration_label = "CPU".to_owned();
        self.batch_size = cpu_batch_size();
        self.last_error = None;
        Ok(())
    }

    fn download_missing_files(
        &self,
        app: &AppHandle,
        progress_state: &RwLock<ModelDownloadProgress>,
    ) -> Result<(), AppError> {
        fs::create_dir_all(&self.cache_dir)?;
        let missing = MODEL_FILES
            .iter()
            .copied()
            .filter(|file| !self.file_is_verified(*file).unwrap_or(false))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(());
        }

        let download_cache = self.cache_dir.join(".hf-cache");
        let mut builder = ApiBuilder::new()
            .with_cache_dir(download_cache.clone())
            .with_progress(false)
            .with_retries(3)
            .with_user_agent("imagyx", env!("CARGO_PKG_VERSION"));
        if let Ok(endpoint) = std::env::var("HF_ENDPOINT") {
            builder = builder.with_endpoint(endpoint);
        }
        let api = builder
            .build()
            .map_err(|error| AppError::Model(error.to_string()))?;
        let repo = api.model(MODEL_REPOSITORY.to_owned());
        let files_with_sizes = missing
            .iter()
            .copied()
            .map(|file| {
                let size = api
                    .metadata(&repo.url(file.remote))
                    .map_err(|error| AppError::Model(error.to_string()))?
                    .size() as u64;
                Ok((file, size))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let total_bytes = files_with_sizes.iter().map(|(_, size)| size).sum();
        let total_files = files_with_sizes.len();
        let mut completed_bytes = 0_u64;

        for (index, (file, file_size)) in files_with_sizes.into_iter().enumerate() {
            let progress = UiDownloadProgress {
                app,
                state: progress_state,
                label: file.label,
                file_name: file.local,
                current_file: index + 1,
                total_files,
                completed_bytes,
                total_bytes,
                current_file_bytes: 0,
                current_file_total: file_size,
                last_emit: Instant::now() - Duration::from_secs(1),
            };
            let downloaded = repo
                .download_with_progress(file.remote, progress)
                .map_err(|error| AppError::Model(error.to_string()))?;
            let destination = self.cache_dir.join(file.local);
            let temporary = destination.with_extension("download");
            fs::copy(&downloaded, &temporary)?;
            if let Some(expected) = file.sha256 {
                if !verify_sha256(&temporary, expected)? {
                    let _ = fs::remove_file(&temporary);
                    return Err(AppError::Model(format!(
                        "empreinte invalide pour {}",
                        file.local
                    )));
                }
            }
            if destination.exists() {
                fs::remove_file(&destination)?;
            }
            fs::rename(&temporary, &destination)?;
            if let Some(expected) = file.sha256 {
                fs::write(verified_marker(&destination), expected)?;
            }
            completed_bytes = completed_bytes.saturating_add(file_size);
        }

        let _ = fs::remove_dir_all(download_cache);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct AccelerationProfile {
    backend: &'static str,
    label: &'static str,
}

impl AccelerationProfile {
    fn provider(self) -> Option<ExecutionProviderDispatch> {
        accelerated_provider()
    }
}

fn acceleration_profile() -> Option<AccelerationProfile> {
    #[cfg(target_os = "macos")]
    {
        Some(AccelerationProfile {
            backend: "CoreML",
            label: "CoreML actif",
        })
    }
    #[cfg(target_os = "windows")]
    {
        Some(AccelerationProfile {
            backend: "DirectML",
            label: "GPU DirectML actif",
        })
    }
    #[cfg(all(target_os = "linux", feature = "nvidia"))]
    {
        Some(AccelerationProfile {
            backend: "CUDA",
            label: "GPU CUDA actif",
        })
    }
    #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
    {
        None
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

fn accelerated_provider() -> Option<ExecutionProviderDispatch> {
    #[cfg(target_os = "macos")]
    {
        Some(ort::ep::CoreML::default().build())
    }
    #[cfg(target_os = "windows")]
    {
        Some(ort::ep::DirectML::default().build())
    }
    #[cfg(all(target_os = "linux", feature = "nvidia"))]
    {
        Some(ort::ep::CUDA::default().build())
    }
    #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
    {
        None
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

fn requested_backend_label() -> &'static str {
    acceleration_profile().map_or("ONNX Runtime CPU", |profile| profile.backend)
}

fn build_session(
    model_path: &Path,
    provider: Option<ExecutionProviderDispatch>,
    disable_cpu_fallback: bool,
    threads: usize,
) -> Result<Session, AppError> {
    let mut builder = Session::builder()
        .map_err(|error| AppError::Model(error.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|error| AppError::Model(error.to_string()))?
        .with_intra_threads(threads)
        .map_err(|error| AppError::Model(error.to_string()))?;
    if let Some(provider) = provider {
        builder = builder
            .with_execution_providers([provider])
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    if disable_cpu_fallback {
        builder = builder
            .with_disable_cpu_fallback()
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    builder
        .commit_from_file(model_path)
        .map_err(|error| AppError::Model(error.to_string()))
}

fn decode_mobileclip_image(path: &Path) -> Result<Vec<f32>, AppError> {
    let image = image::open(path)?.into_rgb8();
    let (width, height) = image.dimensions();
    let shortest = width.min(height).max(1);
    let resized_width =
        ((u64::from(width) * IMAGE_EDGE as u64) / u64::from(shortest)) as u32;
    let resized_height =
        ((u64::from(height) * IMAGE_EDGE as u64) / u64::from(shortest)) as u32;
    let resized = DynamicImage::ImageRgb8(image)
        .resize_exact(resized_width, resized_height, FilterType::CatmullRom)
        .to_rgb8();
    let x = resized_width.saturating_sub(IMAGE_EDGE as u32) / 2;
    let y = resized_height.saturating_sub(IMAGE_EDGE as u32) / 2;
    let cropped = image::imageops::crop_imm(
        &resized,
        x,
        y,
        IMAGE_EDGE as u32,
        IMAGE_EDGE as u32,
    )
    .to_image();
    let plane = IMAGE_EDGE * IMAGE_EDGE;
    let mut output = vec![0.0; plane * 3];
    for (index, pixel) in cropped.pixels().enumerate() {
        output[index] = f32::from(pixel[0]) / 255.0;
        output[plane + index] = f32::from(pixel[1]) / 255.0;
        output[plane * 2 + index] = f32::from(pixel[2]) / 255.0;
    }
    Ok(output)
}

fn normalize(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        vector.iter_mut().for_each(|value| *value /= norm);
    }
    vector
}

fn verified_marker(path: &Path) -> PathBuf {
    path.with_extension(format!(
        "{}.verified",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("model")
    ))
}

fn verify_sha256(path: &Path, expected: &str) -> Result<bool, AppError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()) == expected)
}

fn cpu_batch_size() -> usize {
    (num_cpus::get().max(4) * 2).clamp(8, 16)
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

struct UiDownloadProgress<'a> {
    app: &'a AppHandle,
    state: &'a RwLock<ModelDownloadProgress>,
    label: &'static str,
    file_name: &'static str,
    current_file: usize,
    total_files: usize,
    completed_bytes: u64,
    total_bytes: u64,
    current_file_bytes: u64,
    current_file_total: u64,
    last_emit: Instant,
}

impl UiDownloadProgress<'_> {
    fn emit(&mut self, force: bool) {
        if !force && self.last_emit.elapsed() < Duration::from_millis(80) {
            return;
        }
        self.last_emit = Instant::now();
        let current_bytes = self
            .completed_bytes
            .saturating_add(self.current_file_bytes.min(self.current_file_total))
            .min(self.total_bytes);
        publish_progress(
            self.app,
            self.state,
            ModelDownloadProgress {
                stage: "downloading".to_owned(),
                file_name: Some(self.file_name.to_owned()),
                current_bytes,
                total_bytes: self.total_bytes,
                current_file: self.current_file,
                total_files: self.total_files,
                message: format!("Téléchargement · {}", self.label),
            },
        );
    }
}

impl Progress for UiDownloadProgress<'_> {
    fn init(&mut self, size: usize, _filename: &str) {
        self.current_file_total = size as u64;
        self.current_file_bytes = 0;
        self.emit(true);
    }

    fn update(&mut self, size: usize) {
        self.current_file_bytes = self.current_file_bytes.saturating_add(size as u64);
        self.emit(false);
    }

    fn finish(&mut self) {
        self.current_file_bytes = self.current_file_total;
        self.emit(true);
    }
}

fn publish_progress(
    app: &AppHandle,
    state: &RwLock<ModelDownloadProgress>,
    progress: ModelDownloadProgress,
) {
    *state.write() = progress.clone();
    let _ = app.emit("model-download-progress", progress);
}

fn publish_ready(
    app: &AppHandle,
    progress_state: &RwLock<ModelDownloadProgress>,
    runtime_stats: &RwLock<RuntimeStats>,
    runtime: &MlRuntime,
) {
    publish_progress(
        app,
        progress_state,
        ModelDownloadProgress {
            stage: "ready".to_owned(),
            file_name: None,
            current_bytes: 0,
            total_bytes: 0,
            current_file: MODEL_FILES.len(),
            total_files: MODEL_FILES.len(),
            message: "MobileCLIP2-S0 est prêt hors connexion.".to_owned(),
        },
    );
    {
        let mut stats = runtime_stats.write();
        stats.model_name = MODEL_NAME.to_owned();
        stats.stage = "ready".to_owned();
        stats.backend_requested = runtime.backend_requested.clone();
        stats.backend_effective = runtime.backend_effective.clone();
        stats.acceleration_active = runtime.acceleration_active;
        stats.acceleration_label = runtime.acceleration_label.clone();
        stats.batch_size = runtime.batch_size;
        stats.fallback_reason = runtime.fallback_reason.clone();
    }
    let _ = app.emit("model-status", runtime.status());
    let _ = app.emit("runtime-stats", runtime_stats.read().clone());
}

fn publish_error(
    app: &AppHandle,
    progress_state: &RwLock<ModelDownloadProgress>,
    runtime_stats: &RwLock<RuntimeStats>,
    runtime: &MlRuntime,
    error: &str,
) {
    publish_progress(
        app,
        progress_state,
        ModelDownloadProgress {
            stage: "error".to_owned(),
            file_name: None,
            current_bytes: 0,
            total_bytes: 0,
            current_file: 0,
            total_files: MODEL_FILES.len(),
            message: error.to_owned(),
        },
    );
    runtime_stats.write().stage = "error".to_owned();
    let _ = app.emit("model-status", runtime.status());
}
