use chrono::Utc;
use rusqlite::params;

use crate::{
    AppError,
    ml::MODEL_ID,
    models::VectorEntry,
};

use super::{
    Database,
    codec::{decode_vector, encode_vector},
};

impl Database {
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
