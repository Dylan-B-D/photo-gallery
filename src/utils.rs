use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use image::RgbImage;
use rexif::ExifTag;
use std::borrow::Cow;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{fs, task};
use uuid::Uuid;

use crate::db::{create_image, CreateImageParams};
use crate::types::AppState;

fn uploads_base_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("uploads")
}

pub struct ProcessedImage {
    pub full: Vec<u8>,
    pub optimized: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub original_size: usize,
    pub metadata: ExifMetadata,
}

pub struct SavedImage {
    pub id: i64,
    pub original_filename: String,
    pub original_size: usize,
}

pub struct ExifMetadata {
    pub camera_make: String,
    pub camera_model: String,
    pub lens_model: String,
    pub iso: String,
    pub aperture: String,
    pub shutter_speed: String,
    pub focal_length: String,
    pub light_source: String,
    pub date_created: String,
}

impl ExifMetadata {
    pub fn unknown() -> Self {
        Self {
            camera_make: "Unknown".to_string(),
            camera_model: "Unknown".to_string(),
            lens_model: "Unknown".to_string(),
            iso: "Unknown".to_string(),
            aperture: "Unknown".to_string(),
            shutter_speed: "Unknown".to_string(),
            focal_length: "Unknown".to_string(),
            light_source: "Unknown".to_string(),
            date_created: "Unknown".to_string(),
        }
    }
}

pub enum ImageQuality {
    Full,
    Optimized,
    Thumbnail,
}

impl ImageQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageQuality::Full => "full",
            ImageQuality::Optimized => "optimized",
            ImageQuality::Thumbnail => "thumbnail",
        }
    }
}

pub async fn create_album_directory(album_id: i64) -> io::Result<()> {
    let base_path = uploads_base_dir().join(album_id.to_string());

    // Create directories for each quality
    for quality in [
        ImageQuality::Full,
        ImageQuality::Optimized,
        ImageQuality::Thumbnail,
    ] {
        fs::create_dir_all(base_path.join(quality.as_str())).await?;
    }

    Ok(())
}

pub fn generate_unique_filename(original_filename: &str) -> String {
    let extension = Path::new(original_filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg");

    format!("{}.{}", Uuid::new_v4(), extension)
}

pub async fn save_image(
    file_data: &[u8],
    filename: &str,
    album_id: i64,
    quality: ImageQuality,
) -> io::Result<()> {
    let path = uploads_base_dir()
        .join(album_id.to_string())
        .join(quality.as_str())
        .join(filename);

    fs::write(path, file_data).await
}

pub fn extract_exif_metadata(data: &[u8]) -> Option<ExifMetadata> {
    match rexif::parse_buffer(data) {
        Ok(exif) => {
            let mut camera_make = None;
            let mut camera_model = None;
            let mut lens_model = None;
            let mut iso = None;
            let mut aperture = None;
            let mut shutter_speed = None;
            let mut focal_length = None;
            let mut light_source = None;
            let mut date_created = None;

            for entry in &exif.entries {
                match entry.tag {
                    ExifTag::Make => camera_make = Some(entry.value_more_readable.clone()),
                    ExifTag::Model => camera_model = Some(entry.value_more_readable.clone()),
                    ExifTag::LensModel => lens_model = Some(entry.value_more_readable.clone()),
                    ExifTag::ISOSpeedRatings => iso = Some(entry.value_more_readable.clone()),
                    ExifTag::FNumber => aperture = Some(entry.value_more_readable.clone()),
                    ExifTag::ExposureTime => {
                        shutter_speed = Some(entry.value_more_readable.clone())
                    }
                    ExifTag::FocalLength => focal_length = Some(entry.value_more_readable.clone()),
                    ExifTag::LightSource => light_source = Some(entry.value_more_readable.clone()),
                    ExifTag::DateTimeOriginal => {
                        date_created = Some(entry.value_more_readable.clone())
                    }
                    _ => {}
                }
            }

            Some(ExifMetadata {
                camera_make: camera_make.unwrap_or(Cow::from("Unknown")).to_string(),
                camera_model: camera_model.unwrap_or(Cow::from("Unknown")).to_string(),
                lens_model: lens_model.unwrap_or(Cow::from("Unknown")).to_string(),
                iso: iso.unwrap_or(Cow::from("Unknown")).to_string(),
                aperture: aperture.unwrap_or(Cow::from("Unknown")).to_string(),
                shutter_speed: shutter_speed.unwrap_or(Cow::from("Unknown")).to_string(),
                focal_length: focal_length.unwrap_or(Cow::from("Unknown")).to_string(),
                light_source: light_source.unwrap_or(Cow::from("Unknown")).to_string(),
                date_created: date_created.unwrap_or(Cow::from("Unknown")).to_string(),
            })
        }
        Err(_) => None,
    }
}

pub async fn process_image(data: Vec<u8>) -> Result<ProcessedImage, Box<dyn Error + Send + Sync>> {
    let optimized_max_size = std::env::var("OPTIMIZED_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1920);
    let thumbnail_max_size = std::env::var("THUMBNAIL_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(400);

    let optimized_quality = std::env::var("OPTIMIZED_QUALITY")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(85)
        .clamp(1, 100);
    let thumbnail_quality = std::env::var("THUMBNAIL_QUALITY")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(95)
        .clamp(1, 100);

    task::spawn_blocking(move || {
        let original_size = data.len();

        // EXIF parsing can be expensive or pathological for a malformed image, so
        // keep it off Tokio's async worker threads with the rest of the CPU work.
        let metadata = extract_exif_metadata(&data).unwrap_or_else(ExifMetadata::unknown);

        // Decompress JPEG to RGB image
        let rgb_image: RgbImage = turbojpeg::decompress_image(&data)?;
        let width = rgb_image.width();
        let height = rgb_image.height();

        // Convert to format suitable for fast_image_resize
        let src_image = Image::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3)?;

        // Create resizer with default CPU optimizations
        let mut resizer = Resizer::new();

        // Optimize to max OPTIMIZED_MAX_SIZE
        let (opt_width, opt_height) = calculate_dimensions(width, height, optimized_max_size);
        let mut optimized_img = Image::new(opt_width, opt_height, PixelType::U8x3);

        // Resize the image
        resizer.resize(&src_image, &mut optimized_img, None)?;

        // Create low rez thumbnail
        let (thumb_width, thumb_height) = calculate_dimensions(width, height, thumbnail_max_size);
        let mut thumbnail_img = Image::new(thumb_width, thumb_height, PixelType::U8x3);

        resizer.resize(&src_image, &mut thumbnail_img, None)?;

        // Convert back to RgbImage for compression
        let optimized_rgb =
            RgbImage::from_raw(opt_width, opt_height, optimized_img.buffer().to_vec())
                .ok_or("Failed to create optimized RGB image")?;

        let thumbnail_rgb =
            RgbImage::from_raw(thumb_width, thumb_height, thumbnail_img.buffer().to_vec())
                .ok_or("Failed to create thumbnail RGB image")?;

        // Compress using turbojpeg
        let optimized = turbojpeg::compress_image(
            &optimized_rgb,
            optimized_quality,
            turbojpeg::Subsamp::Sub2x2,
        )?;
        let thumbnail = turbojpeg::compress_image(
            &thumbnail_rgb,
            thumbnail_quality,
            turbojpeg::Subsamp::Sub2x2,
        )?;

        Ok(ProcessedImage {
            full: data,
            optimized: optimized.to_vec(),
            thumbnail: thumbnail.to_vec(),
            original_size,
            metadata,
        })
    })
    .await?
}

fn calculate_dimensions(width: u32, height: u32, max_size: u32) -> (u32, u32) {
    if width <= max_size && height <= max_size {
        return (width, height);
    }

    let ratio = width as f32 / height as f32;
    if width > height {
        let new_width = max_size;
        let new_height = (new_width as f32 / ratio) as u32;
        (new_width, new_height)
    } else {
        let new_height = max_size;
        let new_width = (new_height as f32 * ratio) as u32;
        (new_width, new_height)
    }
}

pub async fn delete_album_directory(album_id: i64) -> io::Result<()> {
    let path = uploads_base_dir().join(album_id.to_string());
    if path.exists() {
        fs::remove_dir_all(path).await?;
    }
    Ok(())
}

pub async fn process_and_save_image(
    state: Arc<AppState>,
    album_id: i64,
    original_filename: String,
    data: Vec<u8>,
) -> Result<SavedImage, Box<dyn Error + Send + Sync>> {
    let original_size = data.len();
    let filename = generate_unique_filename(&original_filename);

    let processed = process_image(data).await?;

    // Do not write anything until decoding and resizing has completed. If a bad
    // image times out or fails, this avoids leaving an orphaned full-size file.
    let save_full = save_image(&processed.full, &filename, album_id, ImageQuality::Full);
    let save_optimized = save_image(
        &processed.optimized,
        &filename,
        album_id,
        ImageQuality::Optimized,
    );
    let save_thumbnail = save_image(
        &processed.thumbnail,
        &filename,
        album_id,
        ImageQuality::Thumbnail,
    );
    tokio::try_join!(save_full, save_optimized, save_thumbnail)?;

    let image_id = create_image(
        &state.pool,
        CreateImageParams {
            album_id,
            filename: &filename,
            file_size: processed.original_size as i64,
            camera_make: &processed.metadata.camera_make,
            camera_model: &processed.metadata.camera_model,
            lens_model: &processed.metadata.lens_model,
            iso: &processed.metadata.iso,
            aperture: &processed.metadata.aperture,
            shutter_speed: &processed.metadata.shutter_speed,
            focal_length: &processed.metadata.focal_length,
            light_source: &processed.metadata.light_source,
            date_created: &processed.metadata.date_created,
        },
    )
    .await?;

    Ok(SavedImage {
        id: image_id,
        original_filename,
        original_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[tokio::test]
    async fn processing_preserves_the_original_and_creates_both_derivatives() {
        let source = RgbImage::from_pixel(8, 4, Rgb([32, 128, 224]));
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode_image(&source)
            .expect("test JPEG encodes");

        let processed = process_image(jpeg.clone())
            .await
            .expect("test JPEG processes");

        assert_eq!(processed.full, jpeg);
        assert_eq!(processed.original_size, processed.full.len());
        assert!(!processed.optimized.is_empty());
        assert!(!processed.thumbnail.is_empty());
    }
}
