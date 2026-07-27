use rusqlite::params;

use crate::{AppError, models::ImageAsset};

use super::{
    Database,
    rows::{IMAGE_COLUMNS, map_image},
};

impl Database {
    pub fn recent_images(
        &self,
        folder_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ImageAsset>, AppError> {
        let connection = self.connect()?;
        let limit = sqlite_limit(limit);
        let offset = sqlite_limit(offset);
        let sql = if folder_id.is_some() {
            format!(
                "SELECT {IMAGE_COLUMNS} FROM images i
                 WHERE i.folder_id = ?1
                 ORDER BY i.modified_at DESC LIMIT ?2 OFFSET ?3"
            )
        } else {
            format!(
                "SELECT {IMAGE_COLUMNS} FROM images i
                 ORDER BY i.modified_at DESC LIMIT ?1 OFFSET ?2"
            )
        };
        let mut statement = connection.prepare(&sql)?;
        let rows = if let Some(folder_id) = folder_id {
            statement.query_map(params![folder_id, limit, offset], map_image)?
        } else {
            statement.query_map(params![limit, offset], map_image)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn pending_images(
        &self,
        folder_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ImageAsset>, AppError> {
        let connection = self.connect()?;
        let limit = sqlite_limit(limit);
        let sql = if folder_id.is_some() {
            format!(
                "SELECT {IMAGE_COLUMNS} FROM images i
                 LEFT JOIN embeddings e ON e.image_id = i.id
                 WHERE e.image_id IS NULL AND i.folder_id = ?1
                 ORDER BY i.modified_at DESC LIMIT ?2"
            )
        } else {
            format!(
                "SELECT {IMAGE_COLUMNS} FROM images i
                 LEFT JOIN embeddings e ON e.image_id = i.id
                 WHERE e.image_id IS NULL
                 ORDER BY i.modified_at DESC LIMIT ?1"
            )
        };
        let mut statement = connection.prepare(&sql)?;
        let rows = if let Some(folder_id) = folder_id {
            statement.query_map(params![folder_id, limit], map_image)?
        } else {
            statement.query_map(params![limit], map_image)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn lexical_search(
        &self,
        fts_query: &str,
        folder_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ImageAsset>, AppError> {
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.connect()?;
        let limit = sqlite_limit(limit);
        let sql = if folder_id.is_some() {
            format!(
                "SELECT {IMAGE_COLUMNS}
                 FROM images_fts JOIN images i ON i.rowid = images_fts.rowid
                 WHERE images_fts MATCH ?1 AND i.folder_id = ?2
                 ORDER BY bm25(images_fts), i.modified_at DESC LIMIT ?3"
            )
        } else {
            format!(
                "SELECT {IMAGE_COLUMNS}
                 FROM images_fts JOIN images i ON i.rowid = images_fts.rowid
                 WHERE images_fts MATCH ?1
                 ORDER BY bm25(images_fts), i.modified_at DESC LIMIT ?2"
            )
        };
        let mut statement = connection.prepare(&sql)?;
        let rows = if let Some(folder_id) = folder_id {
            statement.query_map(params![fts_query, folder_id, limit], map_image)?
        } else {
            statement.query_map(params![fts_query, limit], map_image)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn image_exists(&self, image_id: &str) -> Result<bool, AppError> {
        self.connect()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM images WHERE id = ?1)",
                params![image_id],
                |row| row.get(0),
            )
            .map_err(AppError::from)
    }

    pub fn image_path_is_known(&self, image_id: &str, path: &str) -> Result<bool, AppError> {
        self.connect()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM images WHERE id = ?1 AND path = ?2)",
                params![image_id, path],
                |row| row.get(0),
            )
            .map_err(AppError::from)
    }
}

fn sqlite_limit(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
