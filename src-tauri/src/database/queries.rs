use std::collections::HashMap;

use rusqlite::{params, params_from_iter, types::Value};

use crate::{AppError, models::ImageAsset};

use super::{
    Database,
    rows::{IMAGE_COLUMNS, map_image},
};

impl Database {
    pub fn images_by_ids(&self, ids: &[String]) -> Result<Vec<ImageAsset>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT {IMAGE_COLUMNS} FROM images i WHERE i.id IN ({placeholders})"
        );
        let values = ids.iter().cloned().map(Value::Text).collect::<Vec<_>>();
        let connection = self.connect()?;
        let mut statement = connection.prepare(&sql)?;
        statement
            .query_map(params_from_iter(values), map_image)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
    }

    pub fn folder_ids_for_images(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT id, folder_id FROM images WHERE id IN ({placeholders})");
        let values = ids.iter().cloned().map(Value::Text).collect::<Vec<_>>();
        let connection = self.connect()?;
        let mut statement = connection.prepare(&sql)?;
        statement
            .query_map(params_from_iter(values), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(AppError::from)
    }

    pub fn pending_images(
        &self,
        folder_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ImageAsset>, AppError> {
        let connection = self.connect()?;
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
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::Database;
    use crate::models::{FollowedFolder, ImageAsset};

    fn fixture() -> (tempfile::TempDir, Database, FollowedFolder) {
        let temp = tempdir().expect("temp directory");
        let database = Database::new(temp.path().join("test.sqlite3")).expect("database");
        let folder = FollowedFolder {
            id: "folder".into(),
            name: "Design".into(),
            path: "/tmp/design".into(),
            image_count: 0,
            created_at: 1,
        };
        database.add_folder(&folder).expect("insert folder");
        (temp, database, folder)
    }

    fn asset(folder_id: &str, id: &str, name: &str) -> ImageAsset {
        ImageAsset {
            id: id.into(),
            folder_id: folder_id.into(),
            path: format!("/tmp/design/{name}"),
            name: name.into(),
            extension: "jpg".into(),
            width: 640,
            height: 480,
            size_bytes: 10,
            modified_at: 2,
            thumbnail_path: String::new(),
            semantic_score: None,
        }
    }

    #[test]
    fn fts_finds_filename_prefixes() {
        let (_temp, database, folder) = fixture();
        database
            .save_assets(&[asset(&folder.id, "one", "Green Woman.jpg")], &[])
            .expect("save asset");
        let results = database
            .lexical_search("\"gree\"* AND \"wom\"*", None, 20)
            .expect("fts search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "one");
    }

    #[test]
    fn pending_query_only_returns_missing_embeddings() {
        let (_temp, database, folder) = fixture();
        let first = asset(&folder.id, "one", "one.jpg");
        let second = asset(&folder.id, "two", "two.jpg");
        database
            .save_assets(&[first, second], &[("one".into(), vec![1.0, 0.0])])
            .expect("save assets");
        let pending = database.pending_images(None, 10).expect("pending images");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "two");
    }
}
