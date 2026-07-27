use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard, RwLock};
use tokio::sync::Semaphore;

use crate::{
    AppError,
    database::Database,
    models::{ModelDownloadProgress, RuntimeStats},
    paths::AppPaths,
    system_stats::SystemMonitor,
    thumbnails::ThumbnailCache,
    tracing,
    vector_store::VectorStore,
};

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub thumbnails: ThumbnailCache,
    pub thumbnail_workers: Arc<Semaphore>,
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
        let thumbnail_worker_count = std::thread::available_parallelism()
            .map_or(4, std::num::NonZeroUsize::get)
            .max(2);
        tracing::event(
            "thumbnail.workers.configured",
            format_args!("workers={thumbnail_worker_count}"),
        );

        Ok(Self {
            thumbnails: ThumbnailCache::new(paths.thumbnails.clone())?,
            thumbnail_workers: Arc::new(Semaphore::new(thumbnail_worker_count)),
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
        let store = VectorStore::from_entries(self.database.vectors()?);
        let count = store.len();
        *self.vectors.write() = store;
        tracing::event("startup.vectors.loaded", format_args!("count={count}"));
        Ok(())
    }
}
