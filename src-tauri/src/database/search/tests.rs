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

fn asset(folder_id: &str, id: &str, name: &str, modified_at: i64) -> ImageAsset {
    ImageAsset {
        id: id.into(),
        folder_id: folder_id.into(),
        path: format!("/tmp/design/{name}"),
        name: name.into(),
        extension: "jpg".into(),
        width: 640,
        height: 480,
        size_bytes: 10,
        modified_at,
        thumbnail_path: String::new(),
        semantic_score: None,
    }
}

#[test]
fn fts_finds_filename_prefixes() {
    let (_temp, database, folder) = fixture();
    database
        .save_assets(&[asset(&folder.id, "one", "Green Woman.jpg", 2)], &[])
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
    let first = asset(&folder.id, "one", "one.jpg", 1);
    let second = asset(&folder.id, "two", "two.jpg", 2);
    database
        .save_assets(&[first, second], &[("one".into(), vec![1.0, 0.0])])
        .expect("save assets");
    let pending = database.pending_images(None, 10).expect("pending images");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "two");
}

#[test]
fn recent_query_is_ordered_and_bounded() {
    let (_temp, database, folder) = fixture();
    database
        .save_assets(
            &[
                asset(&folder.id, "old", "old.jpg", 1),
                asset(&folder.id, "new", "new.jpg", 2),
            ],
            &[],
        )
        .expect("save assets");
    let recent = database.recent_images(None, 1).expect("recent images");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "new");
}
