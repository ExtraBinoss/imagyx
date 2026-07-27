use std::{
    collections::{HashMap, VecDeque},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, UNIX_EPOCH},
};

use image::{DynamicImage, ImageReader, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType};
use parking_lot::Mutex;
use serde::Serialize;

use crate::AppError;

const DEFAULT_CAPACITY: usize = 64;
const THUMBNAIL_EDGE: u32 = 256;
pub const AI_IMAGE_EDGE: u32 = 224;
pub const AI_IMAGE_CHANNELS: usize = 3;
const JPEG_QUALITY: u8 = 55;
const WRITE_BUFFER_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPixelProfile {
    pub cache_hit: bool,
    pub lock_wait_ms: f64,
    pub cache_lookup_ms: f64,
    pub thumbnail_read_decode_ms: f64,
    pub source_read_decode_ms: f64,
    pub thumbnail_resize_ms: f64,
    pub thumbnail_encode_write_ms: f64,
    pub ai_resize_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug)]
pub struct PreparedAiPixels {
    pub pixels: Vec<u8>,
    pub profile: AiPixelProfile,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    path: PathBuf,
}

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<String, CacheEntry>,
    lru: VecDeque<String>,
}

#[derive(Debug)]
pub struct ThumbnailCache {
    directory: PathBuf,
    capacity: usize,
    state: Mutex<CacheState>,
    key_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ThumbnailCache {
    pub fn new(directory: PathBuf) -> Result<Self, AppError> {
        Self::with_capacity(directory, DEFAULT_CAPACITY)
    }

    pub fn with_capacity(directory: PathBuf, capacity: usize) -> Result<Self, AppError> {
        fs::create_dir_all(&directory)?;
        let cache = Self {
            directory,
            capacity: capacity.max(1),
            state: Mutex::new(CacheState::default()),
            key_locks: Mutex::new(HashMap::new()),
        };
        cache.load_existing()?;
        Ok(cache)
    }

    pub fn get_or_create(
        &self,
        image_id: &str,
        source: &Path,
        modified_at: i64,
    ) -> Result<PathBuf, AppError> {
        let cache_id = cache_id(image_id, modified_at);
        if let Some(path) = self.get_existing(&cache_id) {
            return Ok(path);
        }

        let lock = self.lock_for(&cache_id);
        let result = {
            let _guard = lock.lock();
            if let Some(path) = self.get_existing(&cache_id) {
                Ok(path)
            } else {
                let path = self.cache_path(&cache_id);
                let thumbnail = create_thumbnail(source)?;
                write_thumbnail(&thumbnail, &path)?;
                self.insert(cache_id.clone(), path.clone());
                Ok(path)
            }
        };
        self.release_lock(&cache_id, &lock);
        result
    }

    #[cfg(test)]
    pub fn prepare_ai_pixels(
        &self,
        image_id: &str,
        source: &Path,
        modified_at: i64,
    ) -> Result<Vec<u8>, AppError> {
        self.prepare_ai_pixels_profiled(image_id, source, modified_at)
            .map(|prepared| prepared.pixels)
    }

    pub fn prepare_ai_pixels_profiled(
        &self,
        image_id: &str,
        source: &Path,
        modified_at: i64,
    ) -> Result<PreparedAiPixels, AppError> {
        let total_started = Instant::now();
        let cache_id = cache_id(image_id, modified_at);
        let mut profile = AiPixelProfile::default();
        let lock_started = Instant::now();
        let lock = self.lock_for(&cache_id);
        let result = {
            let _guard = lock.lock();
            profile.lock_wait_ms = elapsed_ms(lock_started);

            let lookup_started = Instant::now();
            let existing = self.get_existing(&cache_id);
            profile.cache_lookup_ms = elapsed_ms(lookup_started);

            if let Some(path) = existing {
                let decode_started = Instant::now();
                match read_thumbnail_rgb(&path) {
                    Ok(thumbnail) => {
                        profile.cache_hit = true;
                        profile.thumbnail_read_decode_ms = elapsed_ms(decode_started);
                        let resize_started = Instant::now();
                        let pixels = ai_pixels_from_thumbnail(thumbnail);
                        profile.ai_resize_ms = elapsed_ms(resize_started);
                        Ok(pixels)
                    }
                    Err(_error) if !path.is_file() => {
                        profile.thumbnail_read_decode_ms = elapsed_ms(decode_started);
                        let (thumbnail, source_decode_ms, thumbnail_resize_ms) =
                            create_thumbnail_profiled(source)?;
                        profile.source_read_decode_ms = source_decode_ms;
                        profile.thumbnail_resize_ms = thumbnail_resize_ms;
                        let write_started = Instant::now();
                        write_thumbnail(&thumbnail, &path)?;
                        profile.thumbnail_encode_write_ms = elapsed_ms(write_started);
                        self.insert(cache_id.clone(), path);
                        let resize_started = Instant::now();
                        let pixels = ai_pixels_from_thumbnail(thumbnail);
                        profile.ai_resize_ms = elapsed_ms(resize_started);
                        Ok(pixels)
                    }
                    Err(error) => Err(error),
                }
            } else {
                let path = self.cache_path(&cache_id);
                let (thumbnail, source_decode_ms, thumbnail_resize_ms) =
                    create_thumbnail_profiled(source)?;
                profile.source_read_decode_ms = source_decode_ms;
                profile.thumbnail_resize_ms = thumbnail_resize_ms;
                let write_started = Instant::now();
                write_thumbnail(&thumbnail, &path)?;
                profile.thumbnail_encode_write_ms = elapsed_ms(write_started);
                self.insert(cache_id.clone(), path);
                let resize_started = Instant::now();
                let pixels = ai_pixels_from_thumbnail(thumbnail);
                profile.ai_resize_ms = elapsed_ms(resize_started);
                Ok(pixels)
            }
        };
        self.release_lock(&cache_id, &lock);
        profile.total_ms = elapsed_ms(total_started);
        result.map(|pixels| PreparedAiPixels { pixels, profile })
    }

    pub fn cached_items(&self) -> usize {
        self.state.lock().entries.len()
    }

    fn load_existing(&self) -> Result<(), AppError> {
        let mut existing = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jpg") {
                continue;
            }
            let Some(cache_id) = path.file_stem().and_then(|value| value.to_str()) else {
                let _ = fs::remove_file(path);
                continue;
            };
            if cache_id.len() != 64 || !cache_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let _ = fs::remove_file(path);
                continue;
            }
            let modified = entry
                .metadata()?
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_millis());
            existing.push((cache_id.to_owned(), path, modified));
        }
        existing.sort_unstable_by_key(|(_, _, modified)| *modified);

        let overflow = existing.len().saturating_sub(self.capacity);
        for (_, path, _) in existing.iter().take(overflow) {
            let _ = fs::remove_file(path);
        }

        let mut state = self.state.lock();
        for (cache_id, path, _) in existing.into_iter().skip(overflow) {
            state.lru.push_back(cache_id.clone());
            state.entries.insert(cache_id, CacheEntry { path });
        }
        Ok(())
    }

    fn get_existing(&self, cache_id: &str) -> Option<PathBuf> {
        let path = self.cache_path(cache_id);
        if !path.is_file() {
            let mut state = self.state.lock();
            state.entries.remove(cache_id);
            remove_from_lru(&mut state.lru, cache_id);
            return None;
        }

        let mut state = self.state.lock();
        state
            .entries
            .insert(cache_id.to_owned(), CacheEntry { path: path.clone() });
        touch(&mut state.lru, cache_id);
        Some(path)
    }

    fn insert(&self, cache_id: String, path: PathBuf) {
        let mut state = self.state.lock();
        state.entries.insert(cache_id.clone(), CacheEntry { path });
        touch(&mut state.lru, &cache_id);

        while state.entries.len() > self.capacity {
            let Some(oldest) = state.lru.pop_front() else {
                break;
            };
            if let Some(entry) = state.entries.remove(&oldest) {
                let _ = fs::remove_file(entry.path);
            }
        }
    }

    fn cache_path(&self, cache_id: &str) -> PathBuf {
        self.directory.join(format!("{cache_id}.jpg"))
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

fn cache_id(image_id: &str, modified_at: i64) -> String {
    blake3::hash(format!("{image_id}:{modified_at}").as_bytes())
        .to_hex()
        .to_string()
}

fn touch(lru: &mut VecDeque<String>, key: &str) {
    remove_from_lru(lru, key);
    lru.push_back(key.to_owned());
}

fn remove_from_lru(lru: &mut VecDeque<String>, key: &str) {
    if let Some(index) = lru.iter().position(|candidate| candidate == key) {
        lru.remove(index);
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn create_thumbnail(source: &Path) -> Result<RgbImage, AppError> {
    create_thumbnail_profiled(source).map(|(thumbnail, _, _)| thumbnail)
}

fn create_thumbnail_profiled(source: &Path) -> Result<(RgbImage, f64, f64), AppError> {
    let decode_started = Instant::now();
    let image = ImageReader::open(source)?.with_guessed_format()?.decode()?;
    let source_read_decode_ms = elapsed_ms(decode_started);
    let resize_started = Instant::now();
    let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE).to_rgb8();
    let thumbnail_resize_ms = elapsed_ms(resize_started);
    Ok((thumbnail, source_read_decode_ms, thumbnail_resize_ms))
}

fn write_thumbnail(thumbnail: &RgbImage, target: &Path) -> Result<(), AppError> {
    let temporary = target.with_extension(format!("tmp-{}", std::process::id()));
    let write_result = (|| -> Result<(), AppError> {
        let file = File::create(&temporary)?;
        let mut writer = BufWriter::with_capacity(WRITE_BUFFER_BYTES, file);
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, JPEG_QUALITY);
        encoder.encode(
            thumbnail.as_raw(),
            thumbnail.width(),
            thumbnail.height(),
            image::ExtendedColorType::Rgb8,
        )?;
        writer.flush()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    if target.is_file() {
        let _ = fs::remove_file(&temporary);
    } else {
        fs::rename(&temporary, target)?;
    }
    Ok(())
}

fn read_thumbnail_rgb(path: &Path) -> Result<RgbImage, AppError> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgb8())
}

fn ai_pixels_from_thumbnail(thumbnail: RgbImage) -> Vec<u8> {
    DynamicImage::ImageRgb8(thumbnail)
        .resize_to_fill(AI_IMAGE_EDGE, AI_IMAGE_EDGE, FilterType::Triangle)
        .to_rgb8()
        .into_raw()
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use tempfile::tempdir;

    use super::{AI_IMAGE_CHANNELS, AI_IMAGE_EDGE, ThumbnailCache};

    #[test]
    fn keeps_only_the_latest_files() {
        let temp = tempdir().expect("temporary directory");
        let source_dir = temp.path().join("source");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&source_dir).expect("source directory");
        let cache = ThumbnailCache::with_capacity(cache_dir.clone(), 2).expect("cache");

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
        assert_eq!(
            std::fs::read_dir(cache_dir)
                .expect("cache directory")
                .count(),
            2
        );
    }

    #[test]
    fn reuses_the_same_immutable_file() {
        let temp = tempdir().expect("temporary directory");
        let source = temp.path().join("source.png");
        let cache = ThumbnailCache::with_capacity(temp.path().join("cache"), 4).expect("cache");
        RgbImage::from_pixel(64, 64, Rgb([12, 34, 56]))
            .save(&source)
            .expect("source image");

        let first = cache
            .get_or_create("image", &source, 7)
            .expect("first thumbnail");
        let second = cache
            .get_or_create("image", &source, 7)
            .expect("cached thumbnail");

        assert_eq!(first, second);
        assert!(first.is_file());
    }

    #[test]
    fn prepares_fixed_rgb_pixels_for_the_model() {
        let temp = tempdir().expect("temporary directory");
        let source = temp.path().join("source.png");
        let cache = ThumbnailCache::with_capacity(temp.path().join("cache"), 4).expect("cache");
        RgbImage::from_pixel(480, 320, Rgb([12, 34, 56]))
            .save(&source)
            .expect("source image");

        let pixels = cache
            .prepare_ai_pixels("image", &source, 7)
            .expect("AI pixels");

        assert_eq!(
            pixels.len(),
            AI_IMAGE_EDGE as usize * AI_IMAGE_EDGE as usize * AI_IMAGE_CHANNELS
        );
        assert_eq!(cache.cached_items(), 1);
    }
}
