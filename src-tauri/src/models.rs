use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub image_count: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub id: String,
    pub folder_id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub modified_at: i64,
    pub thumbnail_path: String,
    pub semantic_score: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub folder_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub root_dir: String,
    pub models_dir: String,
    pub database_path: String,
    pub thumbnails_dir: String,
    pub ai_backend: String,
    pub ai_ready: bool,
    pub model_progress: ModelDownloadProgress,
    pub runtime_stats: RuntimeStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub folder_id: String,
    pub folder_name: String,
    pub current: usize,
    pub total: usize,
    pub batch_current: Option<usize>,
    pub batch_total: Option<usize>,
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub ready: bool,
    pub backend: String,
    pub acceleration_active: bool,
    pub acceleration_label: String,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub stage: String,
    pub file_name: Option<String>,
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub current_file: usize,
    pub total_files: usize,
    pub message: String,
}

impl Default for ModelDownloadProgress {
    fn default() -> Self {
        Self {
            stage: "idle".to_owned(),
            file_name: None,
            current_bytes: 0,
            total_bytes: 0,
            current_file: 0,
            total_files: 0,
            message: "En attente".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStats {
    pub model_name: String,
    pub stage: String,
    pub backend_requested: String,
    pub backend_effective: String,
    pub acceleration_active: bool,
    pub acceleration_label: String,
    pub batch_size: usize,
    pub current: usize,
    pub total: usize,
    pub batch_current: usize,
    pub batch_total: usize,
    pub images_per_second: f32,
    pub average_ms_per_image: f32,
    pub decode_ms: u64,
    pub inference_ms: u64,
    pub save_ms: u64,
    pub elapsed_ms: u64,
    pub system_cpu_percent: f32,
    pub process_cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub process_memory_bytes: u64,
    pub model_cache_bytes: u64,
    pub thumbnail_cache_items: usize,
    pub fallback_reason: Option<String>,
    pub updated_at: i64,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            model_name: "MobileCLIP2-S0".to_owned(),
            stage: "idle".to_owned(),
            backend_requested: "Détection automatique".to_owned(),
            backend_effective: "En attente".to_owned(),
            acceleration_active: false,
            acceleration_label: "Non initialisée".to_owned(),
            batch_size: 0,
            current: 0,
            total: 0,
            batch_current: 0,
            batch_total: 0,
            images_per_second: 0.0,
            average_ms_per_image: 0.0,
            decode_ms: 0,
            inference_ms: 0,
            save_ms: 0,
            elapsed_ms: 0,
            system_cpu_percent: 0.0,
            process_cpu_percent: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            process_memory_bytes: 0,
            model_cache_bytes: 0,
            thumbnail_cache_items: 0,
            fallback_reason: None,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageFingerprint {
    pub modified_at: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub image_id: String,
    pub folder_id: String,
    pub vector: Vec<f32>,
}
