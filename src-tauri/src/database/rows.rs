use crate::models::{FollowedFolder, ImageAsset};

pub(super) fn map_folder(row: &rusqlite::Row<'_>) -> rusqlite::Result<FollowedFolder> {
    Ok(FollowedFolder {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        created_at: row.get(3)?,
        image_count: row.get::<_, i64>(4)?.try_into().unwrap_or_default(),
    })
}

pub(super) fn map_image(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImageAsset> {
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

pub(super) const IMAGE_COLUMNS: &str = "i.id, i.folder_id, i.path, i.name, i.extension, i.width, i.height, i.size_bytes, \
     i.modified_at, i.thumbnail_path";
