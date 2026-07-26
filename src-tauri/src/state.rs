use parking_lot::{Mutex, MutexGuard, RwLock};
use rusqlite::Connection;

use crate::{
    AppError,
    db::Database,
    ml::MlRuntime,
    models::{ModelDownloadProgress, RuntimeStats, VectorEntry},
    paths::AppPaths,
    thumbnails::ThumbnailCache,
};

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub ml: Mutex<MlRuntime>,
    pub thumbnails: ThumbnailCache,
    pub vectors: RwLock<Vec<VectorEntry>>,
    pub model_progress: RwLock<ModelDownloadProgress>,
    pub runtime_stats: RwLock<RuntimeStats>,
    index_lock: Mutex<()>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Result<Self, AppError> {
        let database = Database::new(paths.database.clone())?;
        migrate_embedding_model(&paths)?;
        let vectors = database.vectors()?;
        Ok(Self {
            ml: Mutex::new(MlRuntime::new(paths.models.clone())),
            thumbnails: ThumbnailCache::new(paths.thumbnails.clone()),
            paths,
            database,
            vectors: RwLock::new(vectors),
            model_progress: RwLock::new(ModelDownloadProgress::default()),
            runtime_stats: RwLock::new(RuntimeStats {
                model_name: "MobileCLIP2-S0".to_owned(),
                backend: "En préparation".to_owned(),
                acceleration: "Non confirmée".to_owned(),
                stage: "idle".to_owned(),
                ..RuntimeStats::default()
            }),
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

fn migrate_embedding_model(paths: &AppPaths) -> Result<(), AppError> {
    let connection = Connection::open(&paths.database)?;
    connection.execute(
        "DELETE FROM embeddings WHERE model != 'mobileclip2-s0'",
        [],
    )?;
    connection.execute_batch(
        "DROP TRIGGER IF EXISTS normalize_embedding_model;
         CREATE TRIGGER normalize_embedding_model
         AFTER INSERT ON embeddings
         BEGIN
           UPDATE embeddings
           SET model = 'mobileclip2-s0'
           WHERE image_id = NEW.image_id;
         END;",
    )?;
    Ok(())
}
