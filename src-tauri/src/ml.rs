use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use fastembed::{
    EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, TextEmbedding,
    TextInitOptions,
};
use hf_hub::{
    Cache,
    api::{Progress, sync::ApiBuilder},
};
use ort::ep::ExecutionProviderDispatch;
use parking_lot::RwLock;
use tauri::{AppHandle, Emitter};

use crate::{
    AppError,
    models::{ModelDownloadProgress, ModelStatus},
};

#[derive(Clone, Copy)]
struct ModelFile {
    repo: &'static str,
    file: &'static str,
    label: &'static str,
}

const MODEL_FILES: [ModelFile; 6] = [
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-vision",
        file: "model.onnx",
        label: "Modèle visuel",
    },
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-text",
        file: "model.onnx",
        label: "Modèle de recherche",
    },
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-text",
        file: "tokenizer.json",
        label: "Vocabulaire",
    },
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-text",
        file: "config.json",
        label: "Configuration",
    },
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-text",
        file: "special_tokens_map.json",
        label: "Tokens spéciaux",
    },
    ModelFile {
        repo: "Qdrant/clip-ViT-B-32-text",
        file: "tokenizer_config.json",
        label: "Configuration du vocabulaire",
    },
];

pub struct MlRuntime {
    cache_dir: PathBuf,
    image_model: Option<ImageEmbedding>,
    text_model: Option<TextEmbedding>,
    last_error: Option<String>,
}

impl std::fmt::Debug for MlRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MlRuntime")
            .field("cache_dir", &self.cache_dir)
            .field("ready", &self.is_ready())
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl MlRuntime {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            image_model: None,
            text_model: None,
            last_error: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.image_model.is_some() && self.text_model.is_some()
    }

    pub fn backend_label() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "CoreML automatique"
        }
        #[cfg(target_os = "windows")]
        {
            "DirectML automatique"
        }
        #[cfg(all(target_os = "linux", feature = "nvidia"))]
        {
            "CUDA automatique"
        }
        #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
        {
            "ONNX Runtime automatique"
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            "ONNX Runtime automatique"
        }
    }

    pub fn prepare(
        &mut self,
        app: &AppHandle,
        progress_state: &RwLock<ModelDownloadProgress>,
    ) -> Result<(), AppError> {
        if self.is_ready() {
            publish_ready(app, progress_state);
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
                message: "Vérification de l’IA locale…".to_owned(),
            },
        );

        if let Err(error) = self.download_missing_files(app, progress_state) {
            self.last_error = Some(error.to_string());
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
                message: format!("Initialisation avec {}…", Self::backend_label()),
            },
        );

        match self.initialize_models() {
            Ok(()) => {
                publish_ready(app, progress_state);
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                publish_error(app, progress_state, &error.to_string());
                Err(error)
            }
        }
    }

    pub fn ensure_ready(&mut self) -> Result<(), AppError> {
        if self.is_ready() {
            return Ok(());
        }
        if !self.cache_has_models() {
            return Err(AppError::Model(
                self.last_error
                    .clone()
                    .unwrap_or_else(|| "L’IA locale n’est pas encore prête".to_owned()),
            ));
        }
        self.initialize_models()
    }

    pub fn embed_images(&mut self, paths: &[PathBuf]) -> Result<Vec<Vec<f32>>, AppError> {
        self.ensure_ready()?;
        let model = self
            .image_model
            .as_mut()
            .ok_or_else(|| AppError::Model("image model unavailable".into()))?;
        model
            .embed(paths, Some(optimal_batch_size()))
            .map_err(|error| AppError::Model(error.to_string()))
    }

    pub fn embed_text(&mut self, query: &str) -> Result<Vec<f32>, AppError> {
        self.ensure_ready()?;
        let model = self
            .text_model
            .as_mut()
            .ok_or_else(|| AppError::Model("text model unavailable".into()))?;
        model
            .embed(vec![query.to_owned()], Some(1))
            .map_err(|error| AppError::Model(error.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Model("empty text embedding".into()))
    }

    pub fn cache_has_models(&self) -> bool {
        let cache = Cache::new(self.cache_dir.clone());
        MODEL_FILES.iter().all(|spec| {
            cache
                .model(spec.repo.to_owned())
                .get(spec.file)
                .is_some()
        })
    }

    fn initialize_models(&mut self) -> Result<(), AppError> {
        let image_options = ImageInitOptions::new(ImageEmbeddingModel::ClipVitB32)
            .with_cache_dir(self.cache_dir.clone())
            .with_show_download_progress(false)
            .with_intra_threads(num_cpus::get())
            .with_execution_providers(execution_providers());
        let text_options = TextInitOptions::new(EmbeddingModel::ClipVitB32)
            .with_cache_dir(self.cache_dir.clone())
            .with_show_download_progress(false)
            .with_intra_threads(num_cpus::get())
            .with_execution_providers(execution_providers());

        match (
            ImageEmbedding::try_new(image_options),
            TextEmbedding::try_new(text_options),
        ) {
            (Ok(image_model), Ok(text_model)) => {
                self.image_model = Some(image_model);
                self.text_model = Some(text_model);
                self.last_error = None;
                Ok(())
            }
            (image_result, text_result) => {
                let message = format!(
                    "image model: {}; text model: {}",
                    image_result
                        .err()
                        .map_or_else(|| "ok".to_owned(), |error| error.to_string()),
                    text_result
                        .err()
                        .map_or_else(|| "ok".to_owned(), |error| error.to_string())
                );
                self.last_error = Some(message.clone());
                Err(AppError::Model(message))
            }
        }
    }

    fn download_missing_files(
        &self,
        app: &AppHandle,
        progress_state: &RwLock<ModelDownloadProgress>,
    ) -> Result<(), AppError> {
        let cache = Cache::new(self.cache_dir.clone());
        let missing: Vec<ModelFile> = MODEL_FILES
            .iter()
            .copied()
            .filter(|spec| {
                cache
                    .model(spec.repo.to_owned())
                    .get(spec.file)
                    .is_none()
            })
            .collect();

        if missing.is_empty() {
            return Ok(());
        }

        let mut builder = ApiBuilder::new()
            .with_cache_dir(self.cache_dir.clone())
            .with_progress(false)
            .with_retries(3)
            .with_user_agent("imagyx", env!("CARGO_PKG_VERSION"));
        if let Ok(endpoint) = std::env::var("HF_ENDPOINT") {
            builder = builder.with_endpoint(endpoint);
        }
        let api = builder
            .build()
            .map_err(|error| AppError::Model(error.to_string()))?;

        let files_with_sizes: Vec<(ModelFile, u64)> = missing
            .iter()
            .copied()
            .map(|spec| {
                let repo = api.model(spec.repo.to_owned());
                let size = api
                    .metadata(&repo.url(spec.file))
                    .map_err(|error| AppError::Model(error.to_string()))?
                    .size() as u64;
                Ok((spec, size))
            })
            .collect::<Result<_, AppError>>()?;
        let total_bytes: u64 = files_with_sizes.iter().map(|(_, size)| size).sum();
        let total_files = files_with_sizes.len();
        let mut completed_bytes = 0_u64;

        for (index, (spec, file_size)) in files_with_sizes.into_iter().enumerate() {
            let repo = api.model(spec.repo.to_owned());
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
                .map_err(|error| AppError::Model(error.to_string()))?;
            completed_bytes = completed_bytes.saturating_add(file_size);
        }

        Ok(())
    }
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

fn publish_ready(app: &AppHandle, state: &RwLock<ModelDownloadProgress>) {
    publish_progress(
        app,
        state,
        ModelDownloadProgress {
            stage: "ready".to_owned(),
            file_name: None,
            current_bytes: 0,
            total_bytes: 0,
            current_file: MODEL_FILES.len(),
            total_files: MODEL_FILES.len(),
            message: "Le modèle est stocké localement et prêt hors connexion.".to_owned(),
        },
    );
    let _ = app.emit(
        "model-status",
        ModelStatus {
            ready: true,
            backend: MlRuntime::backend_label().to_owned(),
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
            backend: format!("{} · recherche par nom", MlRuntime::backend_label()),
        },
    );
}

fn execution_providers() -> Vec<ExecutionProviderDispatch> {
    #[cfg(target_os = "macos")]
    {
        vec![
            ort::ep::CoreML::default().build().fail_silently(),
            ort::ep::CPU::default().build(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            ort::ep::DirectML::default().build().fail_silently(),
            ort::ep::CPU::default().build(),
        ]
    }
    #[cfg(all(target_os = "linux", feature = "nvidia"))]
    {
        vec![
            ort::ep::CUDA::default().build().fail_silently(),
            ort::ep::CPU::default().build(),
        ]
    }
    #[cfg(all(target_os = "linux", not(feature = "nvidia")))]
    {
        vec![ort::ep::CPU::default().build()]
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        vec![ort::ep::CPU::default().build()]
    }
}

fn optimal_batch_size() -> usize {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") || cfg!(feature = "nvidia") {
        32
    } else {
        12
    }
}
