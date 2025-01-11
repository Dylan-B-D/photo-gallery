use sqlx::{SqlitePool, Error, Row};
use crate::models::{ImageMetadata, ModeMetadata};

/// Retrieves the mode (most common value) for specified columns within an album.
pub async fn get_album_mode_metadata(pool: &SqlitePool, album_id: &str) -> Result<ModeMetadata, Error> {
    const MODE_COLUMNS: &[&str] = &["camera_model", "lens_model", "aperture"];

    let mut mode_metadata = ModeMetadata::default();

    for &column in MODE_COLUMNS {
        if !MODE_COLUMNS.contains(&column) {
            continue;
        }

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

        let row = sqlx::query(&query)
            .bind(album_id)
            .fetch_optional(pool)
            .await?;

        match column {
            "camera_model" => {
                mode_metadata.camera_model = row
                    .and_then(|r| r.try_get::<String, _>("camera_model").ok())
                    .map(|s| s.trim_matches('"').to_string());
            }
            "lens_model" => {
                mode_metadata.lens_model = row
                    .and_then(|r| r.try_get::<String, _>("lens_model").ok())
                    .map(|s| s.trim_matches('"').to_string());
            }
            "aperture" => {
                mode_metadata.aperture = row
                    .and_then(|r| r.try_get::<f64, _>("aperture").ok())
                    .map(|a| format!("f/{:.1}", a));
            }
            _ => {}
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
