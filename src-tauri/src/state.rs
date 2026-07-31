use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use parking_lot::{Mutex, MutexGuard, RwLock};
use tokio::sync::Semaphore;

use crate::{
    AppError,
    color_store::ColorStore,
    database::Database,
    fuzzy::FuzzyIndex,
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
    pub fuzzy: RwLock<FuzzyIndex>,
    pub colors: RwLock<ColorStore>,
    pub model_progress: RwLock<ModelDownloadProgress>,
    pub runtime_stats: RwLock<RuntimeStats>,
    pub system_monitor: SystemMonitor,
    index_lock: Mutex<()>,
    model_lock: Mutex<()>,
    search_index_lock: Mutex<()>,
    search_indexes_loaded: AtomicBool,
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
            fuzzy: RwLock::new(FuzzyIndex::default()),
            colors: RwLock::new(ColorStore::default()),
            model_progress: RwLock::new(ModelDownloadProgress::default()),
            runtime_stats: RwLock::new(RuntimeStats::default()),
            system_monitor: SystemMonitor::new(),
            index_lock: Mutex::new(()),
            model_lock: Mutex::new(()),
            search_index_lock: Mutex::new(()),
            search_indexes_loaded: AtomicBool::new(false),
        })
    }

    pub fn lock_indexer(&self) -> MutexGuard<'_, ()> {
        self.index_lock.lock()
    }

    pub fn lock_model(&self) -> MutexGuard<'_, ()> {
        self.model_lock.lock()
    }

    pub fn load_vectors(&self) -> Result<(), AppError> {
        if self.search_indexes_loaded.load(Ordering::Acquire) {
            return Ok(());
        }
        let _guard = self.search_index_lock.lock();
        if self.search_indexes_loaded.load(Ordering::Acquire) {
            return Ok(());
        }
        let store = VectorStore::from_entries(self.database.vectors()?);
        let count = store.len();
        *self.vectors.write() = store;
        let fuzzy = FuzzyIndex::from_entries(self.database.fuzzy_entries()?);
        *self.fuzzy.write() = fuzzy;
        let colors = ColorStore::from_entries(self.database.color_signatures()?);
        let color_count = colors.len();
        *self.colors.write() = colors;
        self.search_indexes_loaded.store(true, Ordering::Release);
        tracing::event(
            "startup.search_indexes.loaded",
            format_args!("vectors={count} colors={color_count}"),
        );
        Ok(())
    }

    pub fn upsert_search_assets(&self, assets: &[crate::models::ImageAsset]) {
        let mut fuzzy = self.fuzzy.write();
        let mut colors = self.colors.write();
        for asset in assets {
            fuzzy.upsert(asset.id.clone(), asset.folder_id.clone(), &asset.name);
            if let Some(signature) = &asset.color_signature {
                colors.upsert(asset.id.clone(), asset.folder_id.clone(), signature.clone());
            } else {
                colors.remove(&asset.id);
            }
        }
    }

    pub fn remove_search_ids<'a>(&self, ids: impl IntoIterator<Item = &'a str>) {
        let ids = ids.into_iter().collect::<Vec<_>>();
        self.fuzzy.write().remove_ids(ids.iter().copied());
        self.colors.write().remove_ids(ids.iter().copied());
    }
}
