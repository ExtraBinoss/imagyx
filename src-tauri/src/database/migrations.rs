use rusqlite::params;

use crate::{AppError, ml::MODEL_ID};

use super::Database;

const FTS_SCHEMA_VERSION: i64 = 1;

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

    let schema_version: i64 = connection.query_row(
        "PRAGMA user_version",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if schema_version < FTS_SCHEMA_VERSION {
        connection.execute("INSERT INTO images_fts(images_fts) VALUES('rebuild')", [])?;
        connection.pragma_update(None, "user_version", FTS_SCHEMA_VERSION)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{Database, FTS_SCHEMA_VERSION};

    #[test]
    fn rebuilds_fts_for_images_created_before_the_migration() {
        let temp = tempdir().expect("temp directory");
        let path = temp.path().join("legacy.sqlite3");
        let connection = Connection::open(&path).expect("legacy database");
        connection
            .execute_batch(
                "CREATE TABLE folders (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    path TEXT NOT NULL UNIQUE,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE images (
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
                INSERT INTO folders (id, name, path, created_at)
                VALUES ('folder', 'Legacy', '/tmp/legacy', 1);
                INSERT INTO images (
                    id, folder_id, path, name, extension, width, height, size_bytes,
                    modified_at, thumbnail_path, search_text, indexed_at
                ) VALUES (
                    'image', 'folder', '/tmp/legacy/legacy-green.jpg',
                    'Legacy Green.jpg', 'jpg', 1, 1, 1, 1, '',
                    'legacy green /tmp/legacy/legacy-green.jpg', 1
                );",
            )
            .expect("legacy schema");
        drop(connection);

        let database = Database::new(path).expect("migrated database");
        let results = database
            .lexical_search("\"legacy\"* AND \"green\"*", None, 10)
            .expect("fts search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "image");

        let version: i64 = database
            .connect()
            .expect("connection")
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version");
        assert_eq!(version, FTS_SCHEMA_VERSION);
    }
}
