use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use image::RgbImage;
use rexif::ExifTag;
use std::borrow::Cow;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use tokio::{fs, task};
use uuid::Uuid;

use crate::handlers::admin::ProcessedImage;

pub enum ImageQuality {
    Full,
    Optimized,
    Thumbnail,
}

impl ImageQuality {
    fn as_str(&self) -> &'static str {
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
    task::spawn_blocking(move || {
        // Decompress JPEG to RGB image
        let rgb_image: RgbImage = turbojpeg::decompress_image(&data)?;
        let width = rgb_image.width();
        let height = rgb_image.height();

        // Convert to format suitable for fast_image_resize
        let src_image = Image::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3)?;

        // Create resizer with default CPU optimizations
        let mut resizer = Resizer::new();

        // Optimize to max 1920x1920
        let (opt_width, opt_height) = calculate_dimensions(width, height, 1920);
        let mut optimized_img = Image::new(opt_width, opt_height, PixelType::U8x3);

        // Resize the image
        resizer.resize(&src_image, &mut optimized_img, None)?;

        // Create thumbnail 400x400
        let (thumb_width, thumb_height) = calculate_dimensions(width, height, 500);
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
        let optimized = turbojpeg::compress_image(&optimized_rgb, 85, turbojpeg::Subsamp::Sub2x2)?;
        let thumbnail = turbojpeg::compress_image(&thumbnail_rgb, 95, turbojpeg::Subsamp::Sub2x2)?;

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