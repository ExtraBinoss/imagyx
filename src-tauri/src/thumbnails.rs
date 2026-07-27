use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageReader, codecs::jpeg::JpegEncoder};
use parking_lot::Mutex;

use crate::AppError;

const DEFAULT_CAPACITY: usize = 4_096;
const PRUNE_INTERVAL: usize = 64;
const THUMBNAIL_EDGE: u32 = 384;
const JPEG_QUALITY: u8 = 76;

#[derive(Debug, Default)]
struct CacheState {
    last_access: HashMap<PathBuf, u64>,
    created_since_prune: usize,
    pruning: bool,
}

#[derive(Debug)]
pub struct ThumbnailCache {
    directory: PathBuf,
    capacity: usize,
    state: Arc<Mutex<CacheState>>,
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
            state: Arc::new(Mutex::new(CacheState::default())),
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
        let result = {
            let _guard = lock.lock();
            if cache_path.exists() {
                self.touch(&cache_path);
                Ok(cache_path.clone())
            } else {
                self.create_thumbnail(source, &cache_path)?;
                self.touch(&cache_path);
                self.schedule_prune(cache_path.clone());
                Ok(cache_path.clone())
            }
        };
        self.release_lock(image_id, &lock);
        result
    }

    pub fn cached_items(&self) -> usize {
        fs::read_dir(&self.directory).map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("jpg")
                })
                .count()
        })
    }

    fn create_thumbnail(&self, source: &Path, cache_path: &Path) -> Result<(), AppError> {
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
            fs::rename(&temporary, cache_path)?;
        }
        Ok(())
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

    fn release_lock(&self, key: &str, lock: &Arc<Mutex<()>>) {
        let mut locks = self.key_locks.lock();
        let is_current = locks
            .get(key)
            .is_some_and(|current| Arc::ptr_eq(current, lock));
        if is_current && Arc::strong_count(lock) == 2 {
            locks.remove(key);
        }
    }

    fn touch(&self, path: &Path) {
        self.state
            .lock()
            .last_access
            .insert(path.to_path_buf(), unix_millis());
    }

    fn schedule_prune(&self, protected: PathBuf) {
        if self.capacity <= PRUNE_INTERVAL {
            let accesses = self.state.lock().last_access.clone();
            if let Ok(removed) = prune_directory(
                &self.directory,
                self.capacity,
                Some(&protected),
                &accesses,
            ) {
                let mut state = self.state.lock();
                for path in removed {
                    state.last_access.remove(&path);
                }
            }
            return;
        }

        let should_prune = {
            let mut state = self.state.lock();
            state.created_since_prune += 1;
            if state.pruning || state.created_since_prune < PRUNE_INTERVAL {
                false
            } else {
                state.created_since_prune = 0;
                state.pruning = true;
                true
            }
        };
        if !should_prune {
            return;
        }

        let directory = self.directory.clone();
        let capacity = self.capacity;
        let state = Arc::clone(&self.state);
        thread::spawn(move || {
            let accesses = state.lock().last_access.clone();
            if let Ok(removed) =
                prune_directory(&directory, capacity, Some(&protected), &accesses)
            {
                let mut cache_state = state.lock();
                for path in removed {
                    cache_state.last_access.remove(&path);
                }
            }
            state.lock().pruning = false;
        });
    }
}

fn prune_directory(
    directory: &Path,
    capacity: usize,
    protected: Option<&Path>,
    accesses: &HashMap<PathBuf, u64>,
) -> Result<Vec<PathBuf>, AppError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory)? {
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

    if entries.len() <= capacity {
        return Ok(Vec::new());
    }

    entries.sort_unstable_by_key(|(_, access)| *access);
    let remove_count = entries.len().saturating_sub(capacity);
    let mut removed = Vec::with_capacity(remove_count);
    for (path, _) in entries.into_iter().take(remove_count) {
        if protected.is_some_and(|protected| protected == path) {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            removed.push(path);
        }
    }
    Ok(removed)
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

        assert_eq!(cache.cached_items(), 2);
    }

    #[test]
    fn reuses_existing_thumbnail_without_reencoding() {
        let temp = tempdir().expect("temporary directory");
        let source = temp.path().join("source.png");
        let cache = ThumbnailCache::with_capacity(temp.path().join("cache"), 4);
        RgbImage::from_pixel(64, 64, Rgb([12, 34, 56]))
            .save(&source)
            .expect("source image");

        let first = cache
            .get_or_create("image", &source, 7)
            .expect("first thumbnail");
        let first_modified = std::fs::metadata(&first)
            .and_then(|metadata| metadata.modified())
            .expect("modified time");
        let second = cache
            .get_or_create("image", &source, 7)
            .expect("cached thumbnail");

        assert_eq!(first, second);
        assert_eq!(
            first_modified,
            std::fs::metadata(second)
                .and_then(|metadata| metadata.modified())
                .expect("cached modified time")
        );
    }
}
