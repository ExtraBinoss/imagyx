use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageReader, codecs::jpeg::JpegEncoder};
use parking_lot::Mutex;

use crate::AppError;

const DEFAULT_CAPACITY: usize = 256;
const THUMBNAIL_EDGE: u32 = 512;
const JPEG_QUALITY: u8 = 80;

#[derive(Debug, Default)]
struct CacheState {
    last_access: HashMap<PathBuf, u64>,
}

#[derive(Debug)]
pub struct ThumbnailCache {
    directory: PathBuf,
    capacity: usize,
    state: Mutex<CacheState>,
    key_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ThumbnailCache {
    pub fn new(directory: PathBuf) -> Self {
        Self::with_capacity(directory, DEFAULT_CAPACITY)
    }

    pub fn with_capacity(directory: PathBuf, capacity: usize) -> Self {
        Self {
            directory,
            capacity: capacity.max(1),
            state: Mutex::new(CacheState::default()),
            key_locks: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_or_create(
        &self,
        image_id: &str,
        source: &Path,
        modified_at: i64,
    ) -> Result<PathBuf, AppError> {
        let cache_path = self.cache_path(image_id, modified_at);
        if cache_path.exists() {
            self.touch(&cache_path);
            return Ok(cache_path);
        }

        let lock = self.lock_for(image_id);
        let _guard = lock.lock();
        if cache_path.exists() {
            self.touch(&cache_path);
            return Ok(cache_path);
        }

        fs::create_dir_all(&self.directory)?;
        let image = ImageReader::open(source)?.with_guessed_format()?.decode()?;
        let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE).to_rgb8();
        let temporary = cache_path.with_extension(format!("tmp-{}", std::process::id()));

        {
            let mut output = File::create(&temporary)?;
            let mut encoder = JpegEncoder::new_with_quality(&mut output, JPEG_QUALITY);
            encoder.encode(
                thumbnail.as_raw(),
                thumbnail.width(),
                thumbnail.height(),
                image::ExtendedColorType::Rgb8,
            )?;
        }

        if cache_path.exists() {
            let _ = fs::remove_file(&temporary);
        } else {
            fs::rename(&temporary, &cache_path)?;
        }

        self.touch(&cache_path);
        self.prune(Some(&cache_path))?;
        Ok(cache_path)
    }

    fn cache_path(&self, image_id: &str, modified_at: i64) -> PathBuf {
        let short_id = image_id.get(..20).unwrap_or(image_id);
        self.directory
            .join(format!("{short_id}-{modified_at}.jpg"))
    }

    fn lock_for(&self, key: &str) -> Arc<Mutex<()>> {
        self.key_locks
            .lock()
            .entry(key.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn touch(&self, path: &Path) {
        self.state
            .lock()
            .last_access
            .insert(path.to_path_buf(), unix_millis());
    }

    fn prune(&self, protected: Option<&Path>) -> Result<(), AppError> {
        let accesses = self.state.lock().last_access.clone();
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jpg") {
                continue;
            }
            let fallback = entry
                .metadata()?
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_millis() as u64);
            entries.push((path.clone(), accesses.get(&path).copied().unwrap_or(fallback)));
        }

        if entries.len() <= self.capacity {
            return Ok(());
        }

        entries.sort_by_key(|(_, access)| *access);
        let remove_count = entries.len().saturating_sub(self.capacity);
        for (path, _) in entries.into_iter().take(remove_count) {
            if protected.is_some_and(|protected| protected == path) {
                continue;
            }
            let _ = fs::remove_file(&path);
            self.state.lock().last_access.remove(&path);
        }
        Ok(())
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use tempfile::tempdir;

    use super::ThumbnailCache;

    #[test]
    fn keeps_the_cache_bounded() {
        let temp = tempdir().expect("temporary directory");
        let source_dir = temp.path().join("source");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&source_dir).expect("source directory");
        let cache = ThumbnailCache::with_capacity(cache_dir.clone(), 2);

        for index in 0..3 {
            let source = source_dir.join(format!("{index}.png"));
            RgbImage::from_pixel(64, 64, Rgb([index as u8, 0, 0]))
                .save(&source)
                .expect("source image");
            cache
                .get_or_create(&format!("{index:064}"), &source, index)
                .expect("thumbnail");
        }

        assert_eq!(
            std::fs::read_dir(cache_dir)
                .expect("cache directory")
                .filter_map(Result::ok)
                .count(),
            2
        );
    }
}
