use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rusqlite::params;

use crate::{AppError, models::ImageAsset};

use super::{Database, embeddings::insert_embeddings};

impl Database {
    pub fn fingerprints_for_folder(
        &self,
        folder_id: &str,
    ) -> Result<HashMap<String, (i64, u64)>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT path, modified_at, size_bytes FROM images WHERE folder_id = ?1",
        )?;
        let rows = statement.query_map(params![folder_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get(1)?,
                    row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
                ),
            ))
        })?;
        rows.collect::<Result<HashMap<_, _>, _>>()
            .map_err(AppError::from)
    }

    pub fn save_assets(
        &self,
        assets: &[ImageAsset],
        embeddings: &[(String, Vec<f32>)],
    ) -> Result<(), AppError> {
        if assets.is_empty() && embeddings.is_empty() {
            return Ok(());
        }
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let now = Utc::now().timestamp_millis();

        {
            let mut statement = transaction.prepare(
                "INSERT INTO images (
                    id, folder_id, path, name, extension, width, height, size_bytes,
                    modified_at, thumbnail_path, search_text, indexed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(path) DO UPDATE SET
                    id = excluded.id,
                    folder_id = excluded.folder_id,
                    name = excluded.name,
                    extension = excluded.extension,
                    width = excluded.width,
                    height = excluded.height,
                    size_bytes = excluded.size_bytes,
                    modified_at = excluded.modified_at,
                    thumbnail_path = excluded.thumbnail_path,
                    search_text = excluded.search_text,
                    indexed_at = excluded.indexed_at",
            )?;
            for asset in assets {
                let search_text = format!(
                    "{} {}",
                    asset.name.to_lowercase(),
                    asset.path.to_lowercase()
                );
                statement.execute(params![
                    asset.id,
                    asset.folder_id,
                    asset.path,
                    asset.name,
                    asset.extension,
                    asset.width,
                    asset.height,
                    i64::try_from(asset.size_bytes).unwrap_or(i64::MAX),
                    asset.modified_at,
                    asset.thumbnail_path,
                    search_text,
                    now,
                ])?;
            }
        }

        let embedded = embeddings
            .iter()
            .map(|(image_id, _)| image_id.as_str())
            .collect::<HashSet<_>>();
        for asset in assets {
            if !embedded.contains(asset.id.as_str()) {
                transaction.execute(
                    "DELETE FROM embeddings WHERE image_id = ?1",
                    params![asset.id],
                )?;
            }
        }
        insert_embeddings(&transaction, embeddings, now)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_missing(
        &self,
        folder_id: &str,
        current: &HashSet<String>,
    ) -> Result<Vec<String>, AppError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS current_scan_paths (
                path TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             DELETE FROM current_scan_paths;",
        )?;
        {
            let mut insert =
                transaction.prepare("INSERT OR IGNORE INTO current_scan_paths(path) VALUES (?1)")?;
            for path in current {
                insert.execute(params![path])?;
            }
        }
        let mut statement = transaction.prepare(
            "DELETE FROM images
             WHERE folder_id = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM current_scan_paths current WHERE current.path = images.path
               )
             RETURNING id",
        )?;
        let deleted = statement
            .query_map(params![folder_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        transaction.commit()?;
        Ok(deleted)
    }
}
