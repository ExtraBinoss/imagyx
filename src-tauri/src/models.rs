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
