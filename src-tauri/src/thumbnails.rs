use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::Arc,
};

use image::{ImageReader, codecs::jpeg::JpegEncoder};
use parking_lot::Mutex;

use crate::AppError;

const DEFAULT_MAX_ITEMS: usize = 768;
const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;
const THUMBNAIL_EDGE: u32 = 384;
const JPEG_QUALITY: u8 = 74;

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<String, Arc<Vec<u8>>>,
    lru: VecDeque<String>,
    bytes: usize,
}

#[derive(Debug)]
pub struct ThumbnailCache {
    max_items: usize,
    max_bytes: usize,
    state: Mutex<CacheState>,
    key_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_ITEMS, DEFAULT_MAX_BYTES)
    }

    pub fn with_limits(max_items: usize, max_bytes: usize) -> Self {
        Self {
            max_items: max_items.max(1),
            max_bytes: max_bytes.max(1),
            state: Mutex::new(CacheState::default()),
            key_locks: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_or_create(
        &self,
        image_id: &str,
        source: &Path,
        modified_at: i64,
    ) -> Result<Arc<Vec<u8>>, AppError> {
        let key = format!("{image_id}:{modified_at}");
        if let Some(bytes) = self.get(&key) {
            return Ok(bytes);
        }

        let lock = self.lock_for(&key);
        let result = {
            let _guard = lock.lock();
            if let Some(bytes) = self.get(&key) {
                Ok(bytes)
            } else {
                let bytes = Arc::new(create_thumbnail(source)?);
                self.insert(key.clone(), Arc::clone(&bytes));
                Ok(bytes)
            }
        };
        self.release_lock(&key, &lock);
        result
    }

    pub fn cached_items(&self) -> usize {
        self.state.lock().entries.len()
    }

    fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        let mut state = self.state.lock();
        let bytes = state.entries.get(key).cloned()?;
        touch(&mut state.lru, key);
        Some(bytes)
    }

    fn insert(&self, key: String, bytes: Arc<Vec<u8>>) {
        if bytes.len() > self.max_bytes {
            return;
        }

        let mut state = self.state.lock();
        if let Some(previous) = state.entries.remove(&key) {
            state.bytes = state.bytes.saturating_sub(previous.len());
        }
        state.bytes = state.bytes.saturating_add(bytes.len());
        state.entries.insert(key.clone(), bytes);
        touch(&mut state.lru, &key);

        while state.entries.len() > self.max_items || state.bytes > self.max_bytes {
            let Some(oldest) = state.lru.pop_front() else {
                break;
            };
            if let Some(removed) = state.entries.remove(&oldest) {
                state.bytes = state.bytes.saturating_sub(removed.len());
            }
        }
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
}

fn touch(lru: &mut VecDeque<String>, key: &str) {
    if let Some(index) = lru.iter().position(|candidate| candidate == key) {
        lru.remove(index);
    }
    lru.push_back(key.to_owned());
}

fn create_thumbnail(source: &Path) -> Result<Vec<u8>, AppError> {
    let image = ImageReader::open(source)?.with_guessed_format()?.decode()?;
    let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE).to_rgb8();
    let mut encoded = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut encoded, JPEG_QUALITY);
    encoder.encode(
        thumbnail.as_raw(),
        thumbnail.width(),
        thumbnail.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use image::{Rgb, RgbImage};
    use tempfile::tempdir;

    use super::ThumbnailCache;

    #[test]
    fn keeps_the_memory_cache_bounded() {
        let temp = tempdir().expect("temporary directory");
        let cache = ThumbnailCache::with_limits(2, usize::MAX);

        for index in 0..3 {
            let source = temp.path().join(format!("{index}.png"));
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
    fn reuses_encoded_bytes_without_writing_a_thumbnail_file() {
        let temp = tempdir().expect("temporary directory");
        let source = temp.path().join("source.png");
        RgbImage::from_pixel(64, 64, Rgb([12, 34, 56]))
            .save(&source)
            .expect("source image");
        let cache = ThumbnailCache::with_limits(4, usize::MAX);

        let first = cache
            .get_or_create("image", &source, 7)
            .expect("first thumbnail");
        let second = cache
            .get_or_create("image", &source, 7)
            .expect("cached thumbnail");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            std::fs::read_dir(temp.path())
                .expect("temporary directory")
                .count(),
            1
        );
    }
}
