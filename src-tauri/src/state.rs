use parking_lot::{Mutex, MutexGuard, RwLock};

use crate::{
    db::Database,
    ml::MlRuntime,
    models::VectorEntry,
    paths::AppPaths,
    AppError,
};

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub ml: Mutex<MlRuntime>,
    pub vectors: RwLock<Vec<VectorEntry>>,
    index_lock: Mutex<()>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Result<Self, AppError> {
        let database = Database::new(paths.database.clone())?;
        let vectors = database.vectors()?;
        Ok(Self {
            ml: Mutex::new(MlRuntime::new(paths.models.clone())),
            paths,
            database,
            vectors: RwLock::new(vectors),
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
