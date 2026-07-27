use rusqlite::{OptionalExtension, params};

use crate::{AppError, models::FollowedFolder};

use super::{Database, rows::map_folder};

impl Database {
    pub fn add_folder(&self, folder: &FollowedFolder) -> Result<(), AppError> {
        self.connect()?.execute(
            "INSERT INTO folders (id, name, path, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![folder.id, folder.name, folder.path, folder.created_at],
        )?;
        Ok(())
    }

    pub fn remove_folder(&self, folder_id: &str) -> Result<(), AppError> {
        self.connect()?
            .execute("DELETE FROM folders WHERE id = ?1", params![folder_id])?;
        Ok(())
    }

    pub fn folder(&self, folder_id: &str) -> Result<Option<FollowedFolder>, AppError> {
        self.connect()?
            .query_row(
                "SELECT f.id, f.name, f.path, f.created_at, COUNT(i.id)
                 FROM folders f LEFT JOIN images i ON i.folder_id = f.id
                 WHERE f.id = ?1 GROUP BY f.id",
                params![folder_id],
                map_folder,
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn folders(&self) -> Result<Vec<FollowedFolder>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT f.id, f.name, f.path, f.created_at, COUNT(i.id)
             FROM folders f LEFT JOIN images i ON i.folder_id = f.id
             GROUP BY f.id ORDER BY lower(f.name)",
        )?;
        statement
            .query_map([], map_folder)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
    }

    pub fn folders_for_watching(&self) -> Result<Vec<FollowedFolder>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, name, path, created_at, 0
             FROM folders ORDER BY lower(name)",
        )?;
        statement
            .query_map([], map_folder)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
    }
}
