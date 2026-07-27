use std::collections::HashMap;

use rusqlite::{params_from_iter, types::Value};

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
        let sql = format!("SELECT {IMAGE_COLUMNS} FROM images i WHERE i.id IN ({placeholders})");
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
}
