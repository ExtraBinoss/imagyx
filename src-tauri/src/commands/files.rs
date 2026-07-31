use std::{
    borrow::Cow,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use arboard::{Clipboard, ImageData};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage, codecs::jpeg::JpegEncoder, imageops};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{indexer, models::ImageAsset, state::AppState};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageConversionFormat {
    Avif,
    Webp,
    Png,
    Jpg,
    Ico,
}

impl ImageConversionFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Avif => "avif",
            Self::Webp => "webp",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Ico => "ico",
        }
    }

    fn image_format(self) -> ImageFormat {
        match self {
            Self::Avif => ImageFormat::Avif,
            Self::Webp => ImageFormat::WebP,
            Self::Png => ImageFormat::Png,
            Self::Jpg => ImageFormat::Jpeg,
            Self::Ico => ImageFormat::Ico,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConversionResult {
    image: ImageAsset,
    inherited_semantic_index: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageConversionProgress {
    source_image_id: String,
    target_format: ImageConversionFormat,
    progress: f32,
    stage: &'static str,
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_file_manager(
    path: String,
    reveal: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let canonical = managed_path(&state, &path)?;
    #[cfg(target_os = "windows")]
    {
        let explorer_path = windows_explorer_path(&canonical);
        let mut command = Command::new("explorer.exe");
        if reveal && canonical.is_file() {
            command.arg("/select,").arg(&explorer_path);
        } else {
            command.arg(&explorer_path);
        }
        command
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir Explorer: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if reveal && canonical.is_file() {
            command.arg("-R");
        }
        command.arg(&canonical);
        command
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir Finder: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if canonical.is_file() {
            canonical.parent().unwrap_or(&canonical)
        } else {
            canonical.as_path()
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir le gestionnaire de fichiers: {error}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_explorer_path(path: &Path) -> PathBuf {
    let path = path.to_string_lossy();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{unc}"));
    }
    let normalized = path.strip_prefix(r"\\?\").unwrap_or(path.as_ref());
    PathBuf::from(normalized)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copy_image_to_clipboard(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let canonical = managed_path(&state, &path)?;
        if !canonical.is_file() {
            return Err("Le chemin ne correspond pas à une image".to_owned());
        }
        let decoded = image::open(&canonical)
            .map_err(|error| format!("Impossible de décoder l’image: {error}"))?
            .into_rgba8();
        let (width, height) = decoded.dimensions();
        let mut clipboard =
            Clipboard::new().map_err(|error| format!("Presse-papiers indisponible: {error}"))?;
        clipboard
            .set_image(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(decoded.into_raw()),
            })
            .map_err(|error| format!("Impossible de copier l’image: {error}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command(rename_all = "camelCase")]
pub async fn convert_image(
    image_id: String,
    target_format: ImageConversionFormat,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<ImageConversionResult, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        convert_image_blocking(&app, &state, &image_id, target_format)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_imagyx(
    image_id: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let image = state
        .database
        .images_by_ids(&[image_id])
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "Image inconnue".to_owned())?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Fenêtre principale indisponible".to_owned())?;
    main.show().map_err(|error| error.to_string())?;
    let _ = main.unminimize();
    main.set_focus().map_err(|error| error.to_string())?;
    main.emit("open-image-requested", image)
        .map_err(|error| error.to_string())?;
    if let Some(spotlight) = app.get_webview_window("spotlight") {
        let _ = spotlight.emit("spotlight-will-hide", ());
        let _ = spotlight.hide();
    }
    Ok(())
}

fn convert_image_blocking(
    app: &AppHandle,
    state: &AppState,
    image_id: &str,
    target_format: ImageConversionFormat,
) -> Result<ImageConversionResult, String> {
    let _index_guard = state.lock_indexer();
    let source = state
        .database
        .images_by_ids(&[image_id.to_owned()])
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "Image inconnue".to_owned())?;
    let source_path = managed_path(state, &source.path)?;
    if normalized_extension(&source.extension) == target_format.extension() {
        return Err("L’image est déjà dans ce format".to_owned());
    }

    emit_conversion_progress(app, image_id, target_format, 0.08, "decoding");
    let decoded = image::open(&source_path)
        .map_err(|error| format!("Impossible de décoder l’image: {error}"))?;
    emit_conversion_progress(app, image_id, target_format, 0.34, "encoding");

    let destination = unique_destination(&source_path, target_format)?;
    let temporary = temporary_path(&destination);
    if let Err(error) = encode_image(&decoded, target_format, &temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, &destination)
        .map_err(|error| format!("Impossible de finaliser l’image convertie: {error}"))?;
    emit_conversion_progress(app, image_id, target_format, 0.82, "indexing");

    let folder = state
        .database
        .folders()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|folder| folder.id == source.folder_id)
        .ok_or_else(|| "Le dossier source n’est plus suivi".to_owned())?;
    let asset = match indexer::prepare_asset(&folder, &destination) {
        Ok(asset) => asset,
        Err(error) => {
            let _ = fs::remove_file(&destination);
            return Err(error.to_string());
        }
    };
    let inherited_embedding = state
        .database
        .embedding_vector(&source.id)
        .map_err(|error| error.to_string())?;
    let embeddings = inherited_embedding
        .as_ref()
        .map(|vector| vec![(asset.id.clone(), vector.clone())])
        .unwrap_or_default();

    if let Err(error) = state
        .database
        .save_assets(std::slice::from_ref(&asset), &embeddings)
    {
        let _ = fs::remove_file(&destination);
        return Err(error.to_string());
    }
    state
        .database
        .link_derivative(&asset.id, &source.id, target_format.extension())
        .map_err(|error| error.to_string())?;
    if let Some(vector) = inherited_embedding {
        state
            .vectors
            .write()
            .upsert([(asset.id.clone(), asset.folder_id.clone(), vector)]);
    }
    state.upsert_search_assets(std::slice::from_ref(&asset));

    emit_conversion_progress(app, image_id, target_format, 1.0, "complete");
    let _ = app.emit("library-updated", ());
    Ok(ImageConversionResult {
        image: asset,
        inherited_semantic_index: !embeddings.is_empty(),
    })
}

fn emit_conversion_progress(
    app: &AppHandle,
    source_image_id: &str,
    target_format: ImageConversionFormat,
    progress: f32,
    stage: &'static str,
) {
    let _ = app.emit(
        "image-conversion-progress",
        ImageConversionProgress {
            source_image_id: source_image_id.to_owned(),
            target_format,
            progress,
            stage,
        },
    );
}

fn encode_image(
    image: &DynamicImage,
    target_format: ImageConversionFormat,
    path: &Path,
) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|error| format!("Impossible de créer l’image convertie: {error}"))?;
    let mut writer = BufWriter::new(file);
    let result = match target_format {
        ImageConversionFormat::Jpg => {
            let flattened = flatten_for_jpeg(image);
            JpegEncoder::new_with_quality(&mut writer, 92)
                .encode_image(&DynamicImage::ImageRgb8(flattened))
        }
        ImageConversionFormat::Ico => {
            prepare_icon(image).write_to(&mut writer, target_format.image_format())
        }
        _ => image.write_to(&mut writer, target_format.image_format()),
    };
    result.map_err(|error| format!("Impossible d’encoder l’image: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("Impossible d’écrire l’image convertie: {error}"))?;
    Ok(())
}

fn flatten_for_jpeg(image: &DynamicImage) -> image::RgbImage {
    let foreground = image.to_rgba8();
    let (width, height) = foreground.dimensions();
    let mut background = RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 255]));
    imageops::overlay(&mut background, &foreground, 0, 0);
    DynamicImage::ImageRgba8(background).into_rgb8()
}

fn prepare_icon(image: &DynamicImage) -> DynamicImage {
    const ICON_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
    let resized = image.thumbnail(256, 256).to_rgba8();
    let (width, height) = resized.dimensions();
    let longest_side = width.max(height);
    let side = ICON_SIZES
        .into_iter()
        .find(|candidate| *candidate >= longest_side)
        .unwrap_or(256);
    let mut canvas = RgbaImage::from_pixel(side, side, Rgba([0, 0, 0, 0]));
    imageops::overlay(
        &mut canvas,
        &resized,
        i64::from((side - width) / 2),
        i64::from((side - height) / 2),
    );
    DynamicImage::ImageRgba8(canvas)
}

fn unique_destination(
    source: &Path,
    target_format: ImageConversionFormat,
) -> Result<PathBuf, String> {
    let parent = source
        .parent()
        .ok_or_else(|| "Le dossier de destination est introuvable".to_owned())?;
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Le nom du fichier source est invalide".to_owned())?;
    let extension = target_format.extension();
    let direct = parent.join(format!("{stem}.{extension}"));
    if !direct.exists() {
        return Ok(direct);
    }
    let converted = parent.join(format!("{stem}-converted.{extension}"));
    if !converted.exists() {
        return Ok(converted);
    }
    for suffix in 2..=9_999 {
        let candidate = parent.join(format!("{stem}-converted-{suffix}.{extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Impossible de trouver un nom libre pour l’image convertie".to_owned())
}

fn temporary_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("converted-image");
    destination.with_file_name(format!(".{file_name}.imagyx-{}", uuid::Uuid::new_v4()))
}

fn normalized_extension(extension: &str) -> &str {
    if extension.eq_ignore_ascii_case("jpeg") {
        "jpg"
    } else {
        extension
    }
}

fn managed_path(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Chemin inaccessible: {error}"))?;
    let folders = state
        .database
        .folders()
        .map_err(|error| error.to_string())?;
    let allowed = folders
        .iter()
        .any(|folder| canonical.starts_with(Path::new(&folder.path)));
    if !allowed {
        return Err("Ce chemin ne fait pas partie d’un dossier suivi".to_owned());
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
    use tempfile::tempdir;

    use super::{ImageConversionFormat, flatten_for_jpeg, prepare_icon, unique_destination};

    #[test]
    fn destination_never_overwrites_an_existing_conversion() {
        let temp = tempdir().expect("temp directory");
        let source = temp.path().join("photo.png");
        fs::write(&source, []).expect("source");
        fs::write(temp.path().join("photo.webp"), []).expect("direct conversion");
        fs::write(temp.path().join("photo-converted.webp"), []).expect("named conversion");

        let destination =
            unique_destination(&source, ImageConversionFormat::Webp).expect("unique destination");
        assert!(destination.ends_with("photo-converted-2.webp"));
    }

    #[test]
    fn icon_output_is_square_and_never_larger_than_256() {
        let source =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(800, 320, Rgba([20, 40, 60, 255])));
        let icon = prepare_icon(&source);
        let (width, height) = icon.dimensions();
        assert_eq!(width, height);
        assert!(width <= 256);
    }

    #[test]
    fn jpeg_conversion_flattens_transparency_on_white() {
        let source = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
        let flattened = flatten_for_jpeg(&source);
        assert_eq!(flattened.get_pixel(0, 0).0, [255, 255, 255]);
    }
}
