use log::error;
use sqlx::{SqlitePool, Error, Row};
use crate::models::{ImageMetadata, ModeMetadata};

/// Retrieves the mode (most common value) for specified columns within an album.
pub async fn get_album_mode_metadata(pool: &SqlitePool, album_id: &str) -> Result<ModeMetadata, Error> {
    // Define the columns for which you want to find the mode.
    const MODE_COLUMNS: &[&str] = &["camera_model", "lens_model", "aperture"];

    // Initialize ModeMetadata with None values.
    let mut mode_metadata = ModeMetadata {
        camera_model: None,
        lens_model: None,
        aperture: None,
    };

    // Iterate over each column and find its mode.
    for &column in MODE_COLUMNS {
        // Validate the column name to prevent SQL injection.
        if !["camera_model", "lens_model", "aperture"].contains(&column) {
            error!("Invalid column name attempted for mode calculation: {}", column);
            continue; // Skip invalid columns.
        }

        // Construct the SQL query safely.
        let query = format!(
            r#"
            SELECT {0}, COUNT(*) as cnt
            FROM image_metadata im
            JOIN images i ON im.image_id = i.id
            WHERE i.album_id = ?1 AND {0} IS NOT NULL
            GROUP BY {0}
            ORDER BY cnt DESC
            LIMIT 1
            "#,
            column
        );

        // Execute the query using `sqlx::query`.
        let row = sqlx::query(&query)
            .bind(album_id)
            .fetch_optional(pool)
            .await?;

        // Extract the mode value from the row, if available.
        let mode_value: Option<String> = row.and_then(|r| r.try_get(column).ok());

        // Assign the mode value to the appropriate field in ModeMetadata.
        match column {
            "camera_model" => mode_metadata.camera_model = mode_value,
            "lens_model" => mode_metadata.lens_model = mode_value,
            "aperture" => mode_metadata.aperture = mode_value,
            _ => {} // Already validated; no action needed.
        }
    }

    Ok(mode_metadata)
}

pub async fn add_image_metadata(
    pool: &SqlitePool,
    metadata: &ImageMetadata
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO image_metadata (
            image_id, camera_make, camera_model, lens_model, 
            iso, aperture, shutter_speed, focal_length, 
            light_source, date_created, file_size
        )
        VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
        )
        "#,
        metadata.image_id, metadata.camera_make, metadata.camera_model,
        metadata.lens_model, metadata.iso, metadata.aperture,
        metadata.shutter_speed, metadata.focal_length,
        metadata.light_source, metadata.date_created, metadata.file_size
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_image_metadata(pool: &SqlitePool, image_id: &str) -> Result<ImageMetadata, Error> {
    let row = sqlx::query!(
        r#"
        SELECT 
            image_id, camera_make, camera_model, lens_model, 
            iso, aperture, shutter_speed, focal_length, 
            light_source, date_created, file_size
        FROM image_metadata
        WHERE image_id = ?1
        "#,
        image_id
    )
    .fetch_one(pool)
    .await?;

    Ok(ImageMetadata {
        image_id: row.image_id.unwrap_or_default(),
        camera_make: row.camera_make,
        camera_model: row.camera_model,
        lens_model: row.lens_model,
        iso: row.iso,
        aperture: row.aperture,
        shutter_speed: row.shutter_speed,
        focal_length: row.focal_length,
        light_source: row.light_source,
        date_created: row.date_created,
        file_size: row.file_size,
    })
}
