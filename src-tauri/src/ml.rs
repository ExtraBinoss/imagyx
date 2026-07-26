use std::path::{Path, PathBuf};

use fastembed::{
    EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, TextEmbedding,
    TextInitOptions,
};
use ort::ep::ExecutionProviderDispatch;

use crate::AppError;

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

    pub fn ensure_ready(&mut self) -> Result<(), AppError> {
        if self.is_ready() {
            return Ok(());
        }

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
        directory_contains_onnx(&self.cache_dir)
    }
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

fn directory_contains_onnx(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        path.extension()
            .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("onnx"))
            || (path.is_dir() && directory_contains_onnx(&path))
    })
}
