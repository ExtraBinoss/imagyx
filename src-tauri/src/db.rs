use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    AppError,
    models::{FollowedFolder, ImageAsset, ImageFingerprint, VectorEntry},
};

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, AppError> {
        let database = Self { path };
        database.migrate()?;
        Ok(database)
    }

    fn connect(&self) -> Result<Connection, AppError> {
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

    fn migrate(&self) -> Result<(), AppError> {
        let connection = self.connect()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS folders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS images (
                id TEXT PRIMARY KEY,
                folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                thumbnail_path TEXT NOT NULL,
                search_text TEXT NOT NULL,
                indexed_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_images_folder ON images(folder_id);
            CREATE INDEX IF NOT EXISTS idx_images_modified ON images(modified_at DESC);
            CREATE INDEX IF NOT EXISTS idx_images_search ON images(search_text);
            CREATE TABLE IF NOT EXISTS embeddings (
                image_id TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
                model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                vector BLOB NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

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
        let connection = self.connect()?;
        connection
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
        let rows = statement.query_map([], map_folder)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn fingerprint(&self, path: &str) -> Result<Option<ImageFingerprint>, AppError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT modified_at, size_bytes FROM images WHERE path = ?1",
                params![path],
                |row| {
                    Ok(ImageFingerprint {
                        modified_at: row.get(0)?,
                        size_bytes: row.get::<_, i64>(1)?.try_into().unwrap_or_default(),
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn save_assets(
        &self,
        assets: &[ImageAsset],
        embeddings: &[(String, Vec<f32>)],
    ) -> Result<(), AppError> {
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

        {
            let embedding_ids: HashSet<&str> = embeddings
                .iter()
                .map(|(image_id, _)| image_id.as_str())
                .collect();
            for asset in assets {
                if !embedding_ids.contains(asset.id.as_str()) {
                    transaction.execute(
                        "DELETE FROM embeddings WHERE image_id = ?1",
                        params![asset.id],
                    )?;
                }
            }

            let mut statement = transaction.prepare(
                "INSERT INTO embeddings (image_id, model, dimensions, vector, updated_at)
                 VALUES (?1, 'clip-vit-b32', ?2, ?3, ?4)
                 ON CONFLICT(image_id) DO UPDATE SET
                    model = excluded.model,
                    dimensions = excluded.dimensions,
                    vector = excluded.vector,
                    updated_at = excluded.updated_at",
            )?;
            for (image_id, vector) in embeddings {
                statement.execute(params![
                    image_id,
                    i64::try_from(vector.len()).unwrap_or_default(),
                    encode_vector(vector),
                    now
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn delete_missing(
        &self,
        folder_id: &str,
        current: &HashSet<String>,
    ) -> Result<(), AppError> {
        let mut connection = self.connect()?;
        let existing = {
            let mut statement =
                connection.prepare("SELECT path FROM images WHERE folder_id = ?1")?;
            let rows = statement.query_map(params![folder_id], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let transaction = connection.transaction()?;
        for path in existing {
            if !current.contains(&path) {
                transaction.execute("DELETE FROM images WHERE path = ?1", params![path])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn images(&self, folder_id: Option<&str>) -> Result<Vec<ImageAsset>, AppError> {
        let connection = self.connect()?;
        let sql = if folder_id.is_some() {
            "SELECT id, folder_id, path, name, extension, width, height, size_bytes,
                    modified_at, thumbnail_path
             FROM images WHERE folder_id = ?1 ORDER BY modified_at DESC"
        } else {
            "SELECT id, folder_id, path, name, extension, width, height, size_bytes,
                    modified_at, thumbnail_path
             FROM images ORDER BY modified_at DESC"
        };
        let mut statement = connection.prepare(sql)?;
        let rows = if let Some(folder_id) = folder_id {
            statement.query_map(params![folder_id], map_image)?
        } else {
            statement.query_map([], map_image)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn vectors(&self) -> Result<Vec<VectorEntry>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT e.image_id, i.folder_id, e.vector
             FROM embeddings e JOIN images i ON i.id = e.image_id",
        )?;
        let rows = statement.query_map([], |row| {
            let blob: Vec<u8> = row.get(2)?;
            Ok(VectorEntry {
                image_id: row.get(0)?,
                folder_id: row.get(1)?,
                vector: decode_vector(&blob),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }
}

fn map_folder(row: &rusqlite::Row<'_>) -> rusqlite::Result<FollowedFolder> {
    Ok(FollowedFolder {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        created_at: row.get(3)?,
        image_count: row.get::<_, i64>(4)?.try_into().unwrap_or_default(),
    })
}

fn map_image(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImageAsset> {
    Ok(ImageAsset {
        id: row.get(0)?,
        folder_id: row.get(1)?,
        path: row.get(2)?,
        name: row.get(3)?,
        extension: row.get(4)?,
        width: row.get::<_, i64>(5)?.try_into().unwrap_or_default(),
        height: row.get::<_, i64>(6)?.try_into().unwrap_or_default(),
        size_bytes: row.get::<_, i64>(7)?.try_into().unwrap_or_default(),
        modified_at: row.get(8)?,
        thumbnail_path: row.get(9)?,
        semantic_score: None,
    })
}

pub fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

pub fn decode_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{Database, decode_vector, encode_vector};
    use crate::models::{FollowedFolder, ImageAsset};

    #[test]
    fn vector_blob_round_trip_is_lossless() {
        let vector = vec![0.25, -1.5, 8.0, f32::EPSILON];
        assert_eq!(decode_vector(&encode_vector(&vector)), vector);
    }

    #[test]
    fn stores_folder_and_image() {
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
        database
            .save_assets(
                &[ImageAsset {
                    id: "image".into(),
                    folder_id: folder.id.clone(),
                    path: "/tmp/design/phone.png".into(),
                    name: "phone.png".into(),
                    extension: "png".into(),
                    width: 100,
                    height: 200,
                    size_bytes: 42,
                    modified_at: 2,
                    thumbnail_path: "/tmp/thumb.jpg".into(),
                    semantic_score: None,
                }],
                &[("image".into(), vec![0.1, 0.2])],
            )
            .expect("save image");
        assert_eq!(database.folders().expect("folders")[0].image_count, 1);
        assert_eq!(
            database.vectors().expect("vectors")[0].vector,
            vec![0.1, 0.2]
        );
    }
}
