mod assets;
mod codec;
mod derivatives;
mod embeddings;
mod folders;
mod migrations;
mod queries;
mod rows;
mod search;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::AppError;

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, AppError> {
        let database = Self { path };
        migrations::migrate(&database)?;
        Ok(database)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn connect(&self) -> Result<Connection, AppError> {
        let connection = Connection::open(&self.path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;",
        )?;
        Ok(connection)
    }
}
