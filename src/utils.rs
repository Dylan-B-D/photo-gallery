use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::IntoResponse;
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

use crate::db::create_image;
use crate::handlers::admin::ProcessedImage;
use crate::types::{AppState, CreateAlbumRequest};

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
    let base_path = PathBuf::from("uploads").join(album_id.to_string());

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
    let path = PathBuf::from("uploads")
        .join(album_id.to_string())
        .join(quality.as_str())
        .join(filename);

    fs::write(path, file_data).await
}

pub fn extract_exif_metadata(data: &[u8]) -> Option<(String, String, String, String, String, String, String, String, String)> {
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
                    ExifTag::ExposureTime => shutter_speed = Some(entry.value_more_readable.clone()),
                    ExifTag::FocalLength => focal_length = Some(entry.value_more_readable.clone()),
                    ExifTag::LightSource => light_source = Some(entry.value_more_readable.clone()),
                    ExifTag::DateTimeOriginal => date_created = Some(entry.value_more_readable.clone()),
                    _ => {}
                }
            }

            Some((
                camera_make.unwrap_or(Cow::from("Unknown")).to_string(),
                camera_model.unwrap_or(Cow::from("Unknown")).to_string(),
                lens_model.unwrap_or(Cow::from("Unknown")).to_string(),
                iso.unwrap_or(Cow::from("Unknown")).to_string(),
                aperture.unwrap_or(Cow::from("Unknown")).to_string(),
                shutter_speed.unwrap_or(Cow::from("Unknown")).to_string(),
                focal_length.unwrap_or(Cow::from("Unknown")).to_string(),
                light_source.unwrap_or(Cow::from("Unknown")).to_string(),
                date_created.unwrap_or(Cow::from("Unknown")).to_string(),
            ))
        }
        Err(_) => None,
    }
}

pub async fn process_image(data: Vec<u8>) -> Result<ProcessedImage, Box<dyn Error + Send + Sync>> {
    const OPTIMIZED_MAX_SIZE: u32 = 1920;
    const THUMBNAIL_MAX_SIZE: u32 = 400;
    const OPTIMIZED_QUALITY: i32 = 85;
    const THUMBNAIL_QUALITY: i32 = 95;

    task::spawn_blocking(move || {
        // Decompress JPEG to RGB image
        let rgb_image: RgbImage = turbojpeg::decompress_image(&data)?;
        let width = rgb_image.width();
        let height = rgb_image.height();

        // Convert to format suitable for fast_image_resize
        let src_image = Image::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3)?;

        // Create resizer with default CPU optimizations
        let mut resizer = Resizer::new();

        // Optimize to max OPTIMIZED_MAX_SIZE
        let (opt_width, opt_height) = calculate_dimensions(width, height, OPTIMIZED_MAX_SIZE);
        let mut optimized_img = Image::new(opt_width, opt_height, PixelType::U8x3);

        // Resize the image
        resizer.resize(&src_image, &mut optimized_img, None)?;

        // Create low rez thumbnail
        let (thumb_width, thumb_height) = calculate_dimensions(width, height, THUMBNAIL_MAX_SIZE);
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
        let optimized = turbojpeg::compress_image(&optimized_rgb, OPTIMIZED_QUALITY, turbojpeg::Subsamp::Sub2x2)?;
        let thumbnail = turbojpeg::compress_image(&thumbnail_rgb, THUMBNAIL_QUALITY, turbojpeg::Subsamp::Sub2x2)?;

        Ok(ProcessedImage {
            optimized: optimized.to_vec(),
            thumbnail: thumbnail.to_vec(),
            original_size: data.len(),
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
    let path = PathBuf::from("uploads").join(album_id.to_string());
    if path.exists() {
        fs::remove_dir_all(path).await?;
    }
    Ok(())
}

pub async fn process_and_save_images(
    state: Arc<AppState>,
    album_id: i64,
    images: Vec<(String, Vec<u8>)>,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let mut tasks = Vec::new();

    for (original_filename, data) in images {
        let state = state.clone();
        let album_id = album_id;
        let filename = generate_unique_filename(&original_filename);

        tasks.push(tokio::spawn(async move {
            // Extract EXIF metadata
            let metadata = extract_exif_metadata(&data).unwrap_or((
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
            ));

            // Save full-resolution image
            save_image(&data, &filename, album_id, ImageQuality::Full).await?;

            // Process the image
            let processed = process_image(data).await?;

            // Save optimized and thumbnail versions
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
            tokio::try_join!(save_optimized, save_thumbnail)?;

            // Create database entry
            create_image(
                &state.pool,
                album_id,
                &filename,
                processed.original_size as i64,
                &metadata.0,
                &metadata.1,
                &metadata.2,
                &metadata.3,
                &metadata.4,
                &metadata.5,
                &metadata.6,
                &metadata.7,
                &metadata.8,
            )
            .await?;

            Ok::<(), Box<dyn Error + Send + Sync>>(())
        }));
    }

    let results = futures::future::join_all(tasks).await;
    Ok(results.into_iter().filter(|r| r.is_ok()).count())
}

/// Extracts multipart fields from the stream.
/// - `album_field`: the field name that contains the album JSON.
/// - `image_field`: the field name that contains image file data.
/// - `deleted_field`: optional field name for a comma‐separated list of deleted image IDs.
pub async fn extract_multipart_fields(
    mut multipart: Multipart,
    album_field: &str,
    image_field: &str,
    deleted_field: Option<&str>,
) -> Result<(Option<CreateAlbumRequest>, Vec<(String, Vec<u8>)>, Vec<i64>), impl IntoResponse> {
    let mut album_data: Option<CreateAlbumRequest> = None;
    let mut images: Vec<(String, Vec<u8>)> = Vec::new();
    let mut deleted_ids: Vec<i64> = Vec::new();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            f if f == album_field => {
                if let Ok(bytes) = field.bytes().await {
                    match serde_json::from_slice(&bytes) {
                        Ok(data) => album_data = Some(data),
                        Err(_) => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                "Invalid album data format",
                            )
                                .into_response())
                        }
                    }
                }
            }
            f if f == image_field => {
                let filename = field
                    .file_name()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown.jpg".to_string());
                let mut file_bytes = Vec::new();
                while let Ok(Some(chunk)) = field.chunk().await {
                    file_bytes.extend_from_slice(&chunk);
                }
                if !file_bytes.is_empty() {
                    images.push((filename, file_bytes));
                }
            }
            f if deleted_field.is_some() && f == deleted_field.unwrap() => {
                if let Ok(bytes) = field.bytes().await {
                    let ids_str = String::from_utf8_lossy(&bytes);
                    deleted_ids = ids_str
                        .split(',')
                        .filter_map(|s| {
                            let trimmed = s.trim();
                            if !trimmed.is_empty() {
                                trimmed.parse::<i64>().ok()
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
            _ => continue,
        }
    }

    Ok((album_data, images, deleted_ids))
}