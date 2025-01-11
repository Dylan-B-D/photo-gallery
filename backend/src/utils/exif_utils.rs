use exif::{Error, Reader, Tag, Value, In};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use crate::models::ImageMetadata;

/// Attempts to parse EXIF metadata from an image file on disk.
/// Returns an `ImageMetadata` struct (without the `image_id` set yet).
// Updated EXIF parser function
pub fn parse_exif_metadata(file_path: &Path) -> Result<ImageMetadata, Error> {
    let file = File::open(file_path)?;
    let file_size = file.metadata().ok().map(|m| m.len() as i64);
    let mut bufreader = BufReader::new(file);

    let exif_reader = Reader::new();
    let exif = exif_reader.read_from_container(&mut bufreader)?;

    // Helper to get a string from a tag
    let get_string = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY)
            .map(|field| field.display_value().with_unit(&exif).to_string())
    };

    // Helper to get an unsigned int
    let get_uint = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY)
            .and_then(|field| field.value.get_uint(0))
            .map(|v| v as i64)
    };

    // Helper to get a rational as f32
    let get_rational_f64 = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY).and_then(|field| {
            match &field.value {
                Value::Rational(ref vals) if !vals.is_empty() => Some(vals[0].to_f64()),
                _ => None,
            }
        })
    };


    Ok(ImageMetadata {
        image_id: String::new(), // Filled in later
        
        // Camera Info
        camera_make: get_string(Tag::Make),
        camera_model: get_string(Tag::Model),
        lens_model: get_string(Tag::LensModel),
        
        // Technical Details
        iso: get_uint(Tag::PhotographicSensitivity)
            .or_else(|| get_uint(Tag::ISOSpeed))
            .or_else(|| get_uint(Tag::StandardOutputSensitivity)),
        aperture: get_rational_f64(Tag::FNumber),
        shutter_speed: get_string(Tag::ExposureTime),
        focal_length: get_rational_f64(Tag::FocalLength),
        
        // Image Details
        light_source: get_string(Tag::LightSource),
        
        // Time and Location
        date_created: get_string(Tag::DateTimeOriginal)
            .or_else(|| get_string(Tag::DateTime))
            .or_else(|| get_string(Tag::DateTimeDigitized)),
    
        // File Info
        file_size,
    })
    
}