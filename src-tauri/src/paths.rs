use std::{fs, path::PathBuf};

use crate::AppError;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub models: PathBuf,
    pub database: PathBuf,
    pub thumbnails: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, AppError> {
        let pictures = dirs::picture_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
            .ok_or(AppError::PicturesDirectoryUnavailable)?;
        Self::from_root(pictures.join("imagyx"))
    }

    pub fn from_root(root: PathBuf) -> Result<Self, AppError> {
        let paths = Self {
            models: root.join("models"),
            database: root.join("database").join("imagyx.sqlite3"),
            thumbnails: root.join("cache").join("thumbnails"),
            logs: root.join("logs"),
            root,
        };
        paths.ensure()?;
        Ok(paths)
    }

    fn ensure(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.models)?;
        fs::create_dir_all(&self.logs)?;
        if let Some(parent) = self.database.parent() {
            fs::create_dir_all(parent)?;
        }

        // Older builds wrote one JPEG preview per image. The UI now loads the
        // source file lazily, so remove those duplicated files during migration.
        if self.thumbnails.exists() {
            fs::remove_dir_all(&self.thumbnails)?;
        }
        fs::create_dir_all(&self.thumbnails)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::AppPaths;

    #[test]
    fn creates_clean_directory_layout() {
        let temp = tempdir().expect("temp directory");
        let root = temp.path().join("imagyx");
        let legacy = root.join("cache").join("thumbnails");
        fs::create_dir_all(&legacy).expect("legacy thumbnail directory");
        fs::write(legacy.join("old.jpg"), b"duplicate").expect("legacy thumbnail");

        let paths = AppPaths::from_root(root).expect("paths");
        assert!(paths.models.is_dir());
        assert!(paths.thumbnails.is_dir());
        assert!(
            fs::read_dir(&paths.thumbnails)
                .expect("thumbnail directory")
                .next()
                .is_none()
        );
        assert_eq!(
            paths.database.file_name().and_then(|name| name.to_str()),
            Some("imagyx.sqlite3")
        );
    }
}
