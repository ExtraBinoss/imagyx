use parking_lot::{Mutex, MutexGuard, RwLock};

use crate::{
    AppError,
    db::Database,
    models::{ModelDownloadProgress, RuntimeStats, VectorEntry},
    paths::AppPaths,
    system_stats::SystemMonitor,
    thumbnails::ThumbnailCache,
};

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub thumbnails: ThumbnailCache,
    pub vectors: RwLock<Vec<VectorEntry>>,
    pub model_progress: RwLock<ModelDownloadProgress>,
    pub runtime_stats: RwLock<RuntimeStats>,
    pub system_monitor: SystemMonitor,
    index_lock: Mutex<()>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Result<Self, AppError> {
        let database = Database::new(paths.database.clone())?;
        let vectors = database.vectors()?;
        Ok(Self {
            thumbnails: ThumbnailCache::new(paths.thumbnails.clone()),
            paths,
            database,
            vectors: RwLock::new(vectors),
            model_progress: RwLock::new(ModelDownloadProgress::default()),
            runtime_stats: RwLock::new(RuntimeStats::default()),
            system_monitor: SystemMonitor::new(),
            index_lock: Mutex::new(()),
        })
    }

    pub fn lock_indexer(&self) -> MutexGuard<'_, ()> {
        self.index_lock.lock()
    }

    pub fn refresh_vectors(&self) -> Result<(), AppError> {
        *self.vectors.write() = self.database.vectors()?;
        Ok(())
    }
}
