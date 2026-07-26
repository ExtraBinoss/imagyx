use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use hf_hub::{
    Cache,
    api::{Progress, sync::ApiBuilder},
};
use image::imageops::FilterType;
use ndarray::Array4;
use ort::{
    ep::ExecutionProviderDispatch,
    session::Session,
    value::Tensor,
};
use parking_lot::RwLock;
use rayon::prelude::*;
use tauri::{AppHandle, Emitter};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::{
    AppError,
    models::{ModelDownloadProgress, ModelStatus, RuntimeStats},
};

const MODEL_REPO: &str = "RuteNL/MobileCLIP2-S0-OpenCLIP-ONNX";
const IMAGE_SIZE: usize = 256;
const CONTEXT_LENGTH: usize = 77;

#[derive(Clone, Copy)]
struct ModelFile {
    file: &'static str,
    label: &'static str,
}

const MODEL_FILES: [ModelFile; 5] = [
    ModelFile {
        file: "visual.onnx",
        label: "Encodeur visuel",
    },
    ModelFile {
        file: "visual.onnx.data",
        label: "Poids visuels",
    },
    ModelFile {
        file: "text.onnx",
        label: "Encodeur texte",
    },
    ModelFile {
        file: "text.onnx.data",
        label: "Poids texte",
    },
    ModelFile {
        file: "tokenizer.json",
        label: "Tokenizer CLIP",
    },
];

pub struct MlRuntime {
    cache_dir: PathBuf,
    image_session: Option<Session>,
    text_session: Option<Session>,
    tokenizer: Option<Tokenizer>,
    backend: String,
    acceleration: String,
    gpu_active: bool,
    last_error: Option<String>,
}

impl std::fmt::Debug for MlRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MlRuntime")
            .field("cache_dir", &self.cache_dir)
            .field("ready", &self.is_ready())
            .field("backend", &self.backend)
            .field("gpu_active", &self.gpu_active)
            .field("last_error", &self.last_error)
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
            backend: "En préparation".to_owned(),
            acceleration: "Non confirmée".to_owned(),
            gpu_active: false,
            last_error: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.image_session.is_some() && self.text_session.is_some() && self.tokenizer.is_some()
    }

    pub fn backend_label(&self) -> &str {
        &self.backend
    }

    pub fn acceleration_label(&self) -> &str {
        &self.acceleration
    }

    pub fn gpu_active(&self) -> bool {
        self.gpu_active
    }

    pub fn batch_size(&self) -> usize {
        if self.gpu_active { 48 } else { 12 }
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

        if let Err(error) = self.download_missing_files(app, progress_state) {
            self.last_error = Some(error.to_string());
            runtime_stats.write().last_error = self.last_error.clone();
            publish_error(app, progress_state, &error.to_string());
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
                message: "Initialisation de MobileCLIP2-S0…".to_owned(),
            },
        );

        match self.initialize_models() {
            Ok(()) => {
                publish_ready(app, progress_state, runtime_stats, self);
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                runtime_stats.write().last_error = self.last_error.clone();
                publish_error(app, progress_state, &error.to_string());
                Err(error)
            }
        }
    }

    pub fn embed_images(&mut self, paths: &[PathBuf]) -> Result<Vec<Vec<f32>>, AppError> {
        self.ensure_ready()?;
        let tensor = preprocess_images(paths)?;
        let input = Tensor::from_array(tensor).map_err(model_error)?;
        let outputs = self
            .image_session
            .as_mut()
            .ok_or_else(|| AppError::Model("encodeur visuel indisponible".into()))?
            .run(ort::inputs![input])
            .map_err(model_error)?;
        vectors_from_output(&outputs[0], paths.len())
    }

    pub fn embed_text(&mut self, query: &str) -> Result<Vec<f32>, AppError> {
        self.ensure_ready()?;
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| AppError::Model("tokenizer indisponible".into()))?;
        let encoding = tokenizer.encode(query, true).map_err(model_error)?;
        let mut ids: Vec<i64> = encoding.get_ids().iter().map(|id| i64::from(*id)).collect();
        ids.resize(CONTEXT_LENGTH, 0);
        ids.truncate(CONTEXT_LENGTH);
        let input = Tensor::from_array(([1_usize, CONTEXT_LENGTH], ids)).map_err(model_error)?;
        let outputs = self
            .text_session
            .as_mut()
            .ok_or_else(|| AppError::Model("encodeur texte indisponible".into()))?
            .run(ort::inputs![input])
            .map_err(model_error)?;
        vectors_from_output(&outputs[0], 1)?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Model("embedding texte vide".into()))
    }

    fn ensure_ready(&mut self) -> Result<(), AppError> {
        if self.is_ready() {
            return Ok(());
        }
        if !self.cache_has_models() {
            return Err(AppError::Model("MobileCLIP2-S0 n’est pas encore prêt".into()));
        }
        self.initialize_models()
    }

    fn initialize_models(&mut self) -> Result<(), AppError> {
        let cache = Cache::new(self.cache_dir.clone());
        let repo = cache.model(MODEL_REPO.to_owned());
        let visual = repo
            .get("visual.onnx")
            .ok_or_else(|| AppError::Model("visual.onnx absent".into()))?;
        let text = repo
            .get("text.onnx")
            .ok_or_else(|| AppError::Model("text.onnx absent".into()))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .ok_or_else(|| AppError::Model("tokenizer.json absent".into()))?;

        let gpu_result = create_sessions(&visual, &text, preferred_execution_providers());
        let (image_session, text_session, backend, acceleration, gpu_active) = match gpu_result {
            Ok((image, text)) if preferred_gpu_available() => (
                image,
                text,
                preferred_backend_name().to_owned(),
                "GPU".to_owned(),
                true,
            ),
            Ok((image, text)) => (
                image,
                text,
                "ONNX Runtime CPU".to_owned(),
                "CPU".to_owned(),
                false,
            ),
            Err(gpu_error) => {
                eprintln!("Imagyx GPU provider unavailable, using CPU: {gpu_error}");
                let (image, text) = create_sessions(&visual, &text, cpu_execution_provider())?;
                (
                    image,
                    text,
                    "ONNX Runtime CPU".to_owned(),
                    "CPU (repli)".to_owned(),
                    false,
                )
            }
        };

        let mut tokenizer = Tokenizer::from_file(tokenizer_path).map_err(model_error)?;
        tokenizer
            .with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::Fixed(CONTEXT_LENGTH),
                ..PaddingParams::default()
            }));
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: CONTEXT_LENGTH,
                ..TruncationParams::default()
            }))
            .map_err(model_error)?;

        self.image_session = Some(image_session);
        self.text_session = Some(text_session);
        self.tokenizer = Some(tokenizer);
        self.backend = backend;
        self.acceleration = acceleration;
        self.gpu_active = gpu_active;
        self.last_error = None;
        Ok(())
    }

    fn cache_has_models(&self) -> bool {
        let cache = Cache::new(self.cache_dir.clone());
        let repo = cache.model(MODEL_REPO.to_owned());
        MODEL_FILES.iter().all(|spec| repo.get(spec.file).is_some())
    }

    fn download_missing_files(
        &self,
        app: &AppHandle,
        progress_state: &RwLock<ModelDownloadProgress>,
    ) -> Result<(), AppError> {
        let cache = Cache::new(self.cache_dir.clone());
        let cached_repo = cache.model(MODEL_REPO.to_owned());
        let missing: Vec<ModelFile> = MODEL_FILES
            .iter()
            .copied()
            .filter(|spec| cached_repo.get(spec.file).is_none())
            .collect();
        if missing.is_empty() {
            return Ok(());
        }

        let api = ApiBuilder::new()
            .with_cache_dir(self.cache_dir.clone())
            .with_progress(false)
            .with_retries(3)
            .with_user_agent("imagyx", env!("CARGO_PKG_VERSION"))
            .build()
            .map_err(model_error)?;
        let repo = api.model(MODEL_REPO.to_owned());
        let files_with_sizes: Vec<(ModelFile, u64)> = missing
            .iter()
            .copied()
            .map(|spec| {
                let size = api
                    .metadata(&repo.url(spec.file))
                    .map_err(model_error)?
                    .size() as u64;
                Ok((spec, size))
            })
            .collect::<Result<_, AppError>>()?;
        let total_bytes = files_with_sizes.iter().map(|(_, size)| size).sum();
        let total_files = files_with_sizes.len();
        let mut completed_bytes = 0_u64;

        for (index, (spec, file_size)) in files_with_sizes.into_iter().enumerate() {
            let progress = UiDownloadProgress {
                app,
                state: progress_state,
                label: spec.label,
                file_name: spec.file,
                current_file: index + 1,
                total_files,
                completed_bytes,
                total_bytes,
                current_file_bytes: 0,
                current_file_total: file_size,
                last_emit: Instant::now() - Duration::from_secs(1),
            };
            repo.download_with_progress(spec.file, progress)
                .map_err(model_error)?;
            completed_bytes = completed_bytes.saturating_add(file_size);
        }
        Ok(())
    }
}

fn create_sessions(
    visual: &Path,
    text: &Path,
    providers: Vec<ExecutionProviderDispatch>,
) -> Result<(Session, Session), AppError> {
    let threads = num_cpus::get().max(1);
    let image_session = Session::builder()
        .map_err(model_error)?
        .with_execution_providers(providers.clone())
        .map_err(model_error)?
        .with_intra_threads(threads)
        .map_err(model_error)?
        .commit_from_file(visual)
        .map_err(model_error)?;
    let text_session = Session::builder()
        .map_err(model_error)?
        .with_execution_providers(providers)
        .map_err(model_error)?
        .with_intra_threads(threads)
        .map_err(model_error)?
        .commit_from_file(text)
        .map_err(model_error)?;
    Ok((image_session, text_session))
}

fn preprocess_images(paths: &[PathBuf]) -> Result<Array4<f32>, AppError> {
    let prepared: Result<Vec<Vec<f32>>, AppError> = paths
        .par_iter()
        .map(|path| {
            let image = image::open(path)?
                .resize_to_fill(IMAGE_SIZE as u32, IMAGE_SIZE as u32, FilterType::Triangle)
                .to_rgb8();
            let mut channels = vec![0_f32; 3 * IMAGE_SIZE * IMAGE_SIZE];
            for (pixel_index, pixel) in image.pixels().enumerate() {
                channels[pixel_index] = f32::from(pixel[0]) / 255.0;
                channels[IMAGE_SIZE * IMAGE_SIZE + pixel_index] = f32::from(pixel[1]) / 255.0;
                channels[2 * IMAGE_SIZE * IMAGE_SIZE + pixel_index] = f32::from(pixel[2]) / 255.0;
            }
            Ok(channels)
        })
        .collect();
    let prepared = prepared?;
    let flat: Vec<f32> = prepared.into_iter().flatten().collect();
    Array4::from_shape_vec((paths.len(), 3, IMAGE_SIZE, IMAGE_SIZE), flat).map_err(model_error)
}

fn vectors_from_output(
    output: &ort::value::DynValue,
    batch: usize,
) -> Result<Vec<Vec<f32>>, AppError> {
    let (_, values) = output.try_extract_tensor::<f32>().map_err(model_error)?;
    if batch == 0 || values.len() % batch != 0 {
        return Err(AppError::Model("forme de sortie MobileCLIP invalide".into()));
    }
    let dimensions = values.len() / batch;
    Ok(values
        .chunks(dimensions)
        .map(|values| normalize(values.to_vec()))
        .collect())
}

fn normalize(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn preferred_execution_providers() -> Vec<ExecutionProviderDispatch> {
    #[cfg(target_os = "macos")]
    {
        vec![ort::ep::CoreML::default().build()]
    }
    #[cfg(target_os = "windows")]
    {
        vec![ort::ep::DirectML::default().build()]
    }
    #[cfg(all(target_os = "linux", feature = "nvidia"))]
    {
        vec![ort::ep::CUDA::default().build()]
    }
    #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
    {
        cpu_execution_provider()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        cpu_execution_provider()
    }
}

fn cpu_execution_provider() -> Vec<ExecutionProviderDispatch> {
    vec![ort::ep::CPU::default().build()]
}

fn preferred_gpu_available() -> bool {
    cfg!(target_os = "macos") || cfg!(target_os = "windows") || cfg!(feature = "nvidia")
}

fn preferred_backend_name() -> &'static str {
    #[cfg(target_os = "macos")]
    { "CoreML" }
    #[cfg(target_os = "windows")]
    { "DirectML" }
    #[cfg(all(target_os = "linux", feature = "nvidia"))]
    { "CUDA" }
    #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
    { "ONNX Runtime CPU" }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { "ONNX Runtime CPU" }
}

fn model_error(error: impl std::fmt::Display) -> AppError {
    AppError::Model(error.to_string())
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
        publish_progress(
            self.app,
            self.state,
            ModelDownloadProgress {
                stage: "downloading".to_owned(),
                file_name: Some(self.file_name.to_owned()),
                current_bytes: self.completed_bytes.saturating_add(self.current_file_bytes),
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
        stats.model_name = "MobileCLIP2-S0".to_owned();
        stats.backend = runtime.backend_label().to_owned();
        stats.acceleration = runtime.acceleration_label().to_owned();
        stats.gpu_active = runtime.gpu_active();
        stats.batch_size = runtime.batch_size();
        stats.stage = "ready".to_owned();
        stats.last_error = None;
    }
    let _ = app.emit(
        "model-status",
        ModelStatus {
            ready: true,
            backend: format!("{} · {}", runtime.backend_label(), runtime.acceleration_label()),
        },
    );
}

fn publish_error(app: &AppHandle, state: &RwLock<ModelDownloadProgress>, error: &str) {
    publish_progress(
        app,
        state,
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
    let _ = app.emit(
        "model-status",
        ModelStatus {
            ready: false,
            backend: "Recherche par nom uniquement".to_owned(),
        },
    );
}
