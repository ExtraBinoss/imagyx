use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::{
    AppError,
    ml::MODEL_ID,
    models::{FolderIndexCoverage, VectorEntry},
};

use super::{
    Database,
    codec::{decode_vector, encode_vector},
};

impl Database {
    pub fn folder_index_coverage(&self) -> Result<Vec<FolderIndexCoverage>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT f.id, COUNT(i.id), COUNT(e.image_id)
             FROM folders f
             LEFT JOIN images i ON i.folder_id = f.id
             LEFT JOIN embeddings e ON e.image_id = i.id AND e.model = ?1
             GROUP BY f.id
             ORDER BY lower(f.name)",
        )?;
        statement
            .query_map(params![MODEL_ID], |row| {
                Ok(FolderIndexCoverage {
                    folder_id: row.get(0)?,
                    image_count: row.get::<_, i64>(1)?.try_into().unwrap_or_default(),
                    embedded_count: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
    }

    pub fn save_embeddings(&self, embeddings: &[(String, Vec<f32>)]) -> Result<(), AppError> {
        if embeddings.is_empty() {
            return Ok(());
        }
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        insert_embeddings(&transaction, embeddings, Utc::now().timestamp_millis())?;
        transaction.commit()?;
        Ok(())
    }

    pub fn embedding_vector(&self, image_id: &str) -> Result<Option<Vec<f32>>, AppError> {
        let connection = self.connect()?;
        let blob = connection
            .query_row(
                "SELECT vector FROM embeddings WHERE image_id = ?1 AND model = ?2",
                params![image_id, MODEL_ID],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()?;
        Ok(blob.map(|value| decode_vector(&value)))
    }

    pub fn vectors(&self) -> Result<Vec<VectorEntry>, AppError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT e.image_id, i.folder_id, e.vector
             FROM embeddings e JOIN images i ON i.id = e.image_id
             WHERE e.model = ?1",
        )?;
        statement
            .query_map(params![MODEL_ID], |row| {
                let blob: Vec<u8> = row.get(2)?;
                Ok(VectorEntry {
                    image_id: row.get(0)?,
                    folder_id: row.get(1)?,
                    vector: decode_vector(&blob),
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
    }

    pub fn reset_embeddings(&self) -> Result<(), AppError> {
        self.connect()?.execute("DELETE FROM embeddings", [])?;
        Ok(())
    }

    pub fn reset_embeddings_for_folder(&self, folder_id: &str) -> Result<usize, AppError> {
        self.connect()?
            .execute(
                "DELETE FROM embeddings
                 WHERE image_id IN (SELECT id FROM images WHERE folder_id = ?1)",
                params![folder_id],
            )
            .map_err(AppError::from)
    }
}

pub(super) fn insert_embeddings(
    transaction: &rusqlite::Transaction<'_>,
    embeddings: &[(String, Vec<f32>)],
    now: i64,
) -> Result<(), AppError> {
    let mut statement = transaction.prepare(
        "INSERT INTO embeddings (image_id, model, dimensions, vector, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(image_id) DO UPDATE SET
            model = excluded.model,
            dimensions = excluded.dimensions,
            vector = excluded.vector,
            updated_at = excluded.updated_at",
    )?;
    for (image_id, vector) in embeddings {
        statement.execute(params![
            image_id,
            MODEL_ID,
            i64::try_from(vector.len()).unwrap_or_default(),
            encode_vector(vector),
            now
        ])?;
    }
    Ok(())
}
