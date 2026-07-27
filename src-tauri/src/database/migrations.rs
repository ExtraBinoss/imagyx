use rusqlite::params;

use crate::{AppError, ml::MODEL_ID};

use super::Database;

pub(super) fn migrate(database: &Database) -> Result<(), AppError> {
    let connection = database.connect()?;
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
        CREATE TABLE IF NOT EXISTS embeddings (
            image_id TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
            model TEXT NOT NULL,
            dimensions INTEGER NOT NULL,
            vector BLOB NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model);
        CREATE VIRTUAL TABLE IF NOT EXISTS images_fts USING fts5(
            name,
            path,
            search_text,
            content='images',
            content_rowid='rowid',
            tokenize='unicode61 remove_diacritics 2',
            prefix='2 3 4'
        );
        CREATE TRIGGER IF NOT EXISTS images_fts_insert AFTER INSERT ON images BEGIN
            INSERT INTO images_fts(rowid, name, path, search_text)
            VALUES (new.rowid, new.name, new.path, new.search_text);
        END;
        CREATE TRIGGER IF NOT EXISTS images_fts_delete AFTER DELETE ON images BEGIN
            INSERT INTO images_fts(images_fts, rowid, name, path, search_text)
            VALUES ('delete', old.rowid, old.name, old.path, old.search_text);
        END;
        CREATE TRIGGER IF NOT EXISTS images_fts_update AFTER UPDATE ON images BEGIN
            INSERT INTO images_fts(images_fts, rowid, name, path, search_text)
            VALUES ('delete', old.rowid, old.name, old.path, old.search_text);
            INSERT INTO images_fts(rowid, name, path, search_text)
            VALUES (new.rowid, new.name, new.path, new.search_text);
        END;",
    )?;

    connection.execute(
        "DELETE FROM embeddings WHERE model <> ?1",
        params![MODEL_ID],
    )?;

    let image_count: i64 = connection.query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))?;
    let fts_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM images_fts", [], |row| row.get(0))?;
    if image_count != fts_count {
        connection.execute("INSERT INTO images_fts(images_fts) VALUES('rebuild')", [])?;
    }

    Ok(())
}
