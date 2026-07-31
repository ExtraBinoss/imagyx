use chrono::Utc;
use rusqlite::params;

use crate::AppError;

use super::Database;

impl Database {
    pub fn link_derivative(
        &self,
        image_id: &str,
        source_image_id: &str,
        conversion_format: &str,
    ) -> Result<(), AppError> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO image_derivatives (
                image_id, source_image_id, conversion_format, created_at
             ) VALUES (
                ?1,
                COALESCE(
                    (SELECT source_image_id FROM image_derivatives WHERE image_id = ?2),
                    ?2
                ),
                ?3,
                ?4
             )
             ON CONFLICT(image_id) DO UPDATE SET
                source_image_id = excluded.source_image_id,
                conversion_format = excluded.conversion_format,
                created_at = excluded.created_at",
            params![
                image_id,
                source_image_id,
                conversion_format,
                Utc::now().timestamp_millis()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{database::Database, models::ImageAsset};

    #[test]
    fn keeps_converted_images_in_the_same_derivative_family() {
        let temp = tempdir().expect("temp directory");
        let database = Database::new(temp.path().join("imagyx.sqlite3")).expect("database");
        let folder_path = temp.path().join("images");
        fs::create_dir_all(&folder_path).expect("images directory");
        let connection = database.connect().expect("connection");
        connection
            .execute(
                "INSERT INTO folders (id, name, path, created_at) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params!["folder", "Images", folder_path.to_string_lossy()],
            )
            .expect("folder");
        drop(connection);

        let asset = |id: &str, extension: &str| ImageAsset {
            id: id.to_owned(),
            folder_id: "folder".to_owned(),
            path: folder_path
                .join(format!("{id}.{extension}"))
                .to_string_lossy()
                .into_owned(),
            name: format!("{id}.{extension}"),
            extension: extension.to_owned(),
            width: 1,
            height: 1,
            size_bytes: 1,
            modified_at: 1,
            thumbnail_path: String::new(),
            color_signature: None,
            semantic_score: None,
            relevance_score: None,
        };
        database
            .save_assets(
                &[
                    asset("source", "png"),
                    asset("webp", "webp"),
                    asset("avif", "avif"),
                ],
                &[],
            )
            .expect("assets");
        database
            .link_derivative("webp", "source", "webp")
            .expect("first derivative");
        database
            .link_derivative("avif", "webp", "avif")
            .expect("second derivative");

        let root: String = database
            .connect()
            .expect("connection")
            .query_row(
                "SELECT source_image_id FROM image_derivatives WHERE image_id = 'avif'",
                [],
                |row| row.get(0),
            )
            .expect("derivative root");
        assert_eq!(root, "source");
    }
}
