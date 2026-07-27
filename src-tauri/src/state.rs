use parking_lot::{Mutex, MutexGuard, RwLock};

use crate::{
    AppError,
    database::Database,
    models::{ModelDownloadProgress, RuntimeStats},
    paths::AppPaths,
    system_stats::SystemMonitor,
    thumbnails::ThumbnailCache,
    vector_store::VectorStore,
};

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub thumbnails: ThumbnailCache,
    pub vectors: RwLock<VectorStore>,
    pub model_progress: RwLock<ModelDownloadProgress>,
    pub runtime_stats: RwLock<RuntimeStats>,
    pub system_monitor: SystemMonitor,
    index_lock: Mutex<()>,
    model_lock: Mutex<()>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Result<Self, AppError> {
        let database = Database::new(paths.database.clone())?;
        Ok(Self {
            thumbnails: ThumbnailCache::new(paths.thumbnails.clone()),
            paths,
            database,
            vectors: RwLock::new(VectorStore::default()),
            model_progress: RwLock::new(ModelDownloadProgress::default()),
            runtime_stats: RwLock::new(RuntimeStats::default()),
            system_monitor: SystemMonitor::new(),
            index_lock: Mutex::new(()),
            model_lock: Mutex::new(()),
        })
    }

    pub fn lock_indexer(&self) -> MutexGuard<'_, ()> {
        self.index_lock.lock()
    }

    pub fn lock_model(&self) -> MutexGuard<'_, ()> {
        self.model_lock.lock()
    }

    pub fn load_vectors(&self) -> Result<(), AppError> {
        let vectors = self.database.vectors()?;
        self.vectors.write().upsert(
            vectors
                .into_iter()
                .map(|entry| (entry.image_id, entry.folder_id, entry.vector)),
        );
        Ok(())
    }
}
