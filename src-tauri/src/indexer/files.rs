use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use walkdir::WalkDir;

use crate::{
    AppError,
    models::{FollowedFolder, ImageAsset},
};

pub(super) fn discover_images(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| is_supported_image(path))
        .collect()
}

pub(super) fn is_changed(path: &Path, existing: &HashMap<String, (i64, u64)>) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    let fingerprint = (modified_millis(&metadata), metadata.len());
    existing
        .get(path.to_string_lossy().as_ref())
        .is_none_or(|existing| *existing != fingerprint)
}

pub(crate) fn prepare_asset(
    folder: &FollowedFolder,
    path: &Path,
) -> Result<ImageAsset, AppError> {
    let metadata = path.metadata()?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::InvalidPath(path.to_path_buf()))?
        .to_owned();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let path_string = path.to_string_lossy().into_owned();
    let id = blake3::hash(path_string.as_bytes()).to_hex().to_string();
    let (width, height) = image::image_dimensions(path)?;
    Ok(ImageAsset {
        id,
        folder_id: folder.id.clone(),
        path: path_string,
        name: file_name,
        extension,
        width,
        height,
        size_bytes: metadata.len(),
        modified_at: modified_millis(&metadata),
        thumbnail_path: String::new(),
        semantic_score: None,
    })
}

fn modified_millis(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "avif"
                    | "jpg"
                    | "jpeg"
                    | "png"
                    | "webp"
                    | "gif"
                    | "bmp"
                    | "tif"
                    | "tiff"
                    | "ico"
            )
        })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, path::Path};

    use tempfile::tempdir;

    use super::{discover_images, is_changed, is_supported_image};

    #[test]
    fn recognises_supported_extensions_case_insensitively() {
        assert!(is_supported_image(Path::new("photo.AVIF")));
        assert!(is_supported_image(Path::new("photo.JPEG")));
        assert!(is_supported_image(Path::new("icon.ico")));
        assert!(!is_supported_image(Path::new("notes.txt")));
    }

    #[test]
    fn discovery_ignores_non_images() {
        let temp = tempdir().expect("temp directory");
        fs::write(temp.path().join("image.png"), []).expect("image fixture");
        fs::write(temp.path().join("notes.txt"), []).expect("text fixture");
        let discovered = discover_images(temp.path());
        assert_eq!(discovered.len(), 1);
        assert!(discovered[0].ends_with("image.png"));
    }

    #[test]
    fn missing_fingerprint_marks_a_file_as_changed() {
        let temp = tempdir().expect("temp directory");
        let path = temp.path().join("image.png");
        fs::write(&path, []).expect("fixture");
        assert!(is_changed(&path, &HashMap::new()));
    }
}
