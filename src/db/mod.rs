use crate::types::{Album, CreateAlbumRequest, Image};
use sqlx::SqlitePool;

pub async fn create_album(
    pool: &SqlitePool,
    album: &CreateAlbumRequest,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO albums (name, description, date, num_images)
        VALUES (?, ?, ?, 0)
        "#,
        album.name,
        album.description,
        album.date,
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn create_image(
    pool: &SqlitePool,
    album_id: i64,
    filename: &str,
    file_size: i64,
    camera_make: &str,
    camera_model: &str,
    lens_model: &str,
    iso: &str,
    aperture: &str,
    shutter_speed: &str,
    focal_length: &str,
    light_source: &str,
    date_created: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO images (
            album_id, filename, file_size, 
            camera_make, camera_model, lens_model, 
            iso, aperture, shutter_speed, focal_length, 
            light_source, date_created
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        album_id,
        filename,
        file_size,
        camera_make,
        camera_model,
        lens_model,
        iso,
        aperture,
        shutter_speed,
        focal_length,
        light_source,
        date_created,
    )
    .execute(pool)
    .await?;

    // Update the number of images in the album
    sqlx::query!(
        "UPDATE albums SET num_images = num_images + 1 WHERE id = ?",
        album_id
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_album_metadata(pool: &SqlitePool, album_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE albums 
        SET camera_model = (
            SELECT camera_model 
            FROM images 
            WHERE album_id = ? 
            GROUP BY camera_model 
            ORDER BY COUNT(*) DESC 
            LIMIT 1
        ),
        lens_model = (
            SELECT lens_model 
            FROM images 
            WHERE album_id = ? 
            GROUP BY lens_model 
            ORDER BY COUNT(*) DESC 
            LIMIT 1
        ),
        aperture = (
            SELECT aperture 
            FROM images 
            WHERE album_id = ? 
            GROUP BY aperture 
            ORDER BY COUNT(*) DESC 
            LIMIT 1
        )
        WHERE id = ?
        "#,
        album_id,
        album_id,
        album_id,
        album_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_albums_with_oldest_image(
    pool: &SqlitePool,
) -> Result<Vec<(Album, Option<String>)>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct AlbumWithImage {
        id: i64,
        name: String,
        description: Option<String>,
        date: String,
        num_images: Option<i64>,
        camera_model: Option<String>,
        lens_model: Option<String>,
        aperture: Option<String>,
        oldest_image: Option<String>,
    }

    let results = sqlx::query_as!(
        AlbumWithImage,
        r#"
        SELECT 
            a.id, 
            a.name, 
            a.description, 
            a.date, 
            a.num_images,
            a.camera_model, 
            a.lens_model, 
            a.aperture,
            (
                SELECT i.filename
                FROM images i
                WHERE i.album_id = a.id
                ORDER BY i.date_created ASC
                LIMIT 1
            ) as oldest_image
        FROM albums a
        ORDER BY a.date DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(results
        .into_iter()
        .map(|r| {
            (
                Album {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    date: r.date,
                    num_images: r.num_images.unwrap_or(0) as i32,
                    camera_model: r.camera_model,
                    lens_model: r.lens_model,
                    aperture: r.aperture,
                },
                r.oldest_image,
            )
        })
        .collect())
}

pub async fn get_album_with_images(
    pool: &SqlitePool,
    album_id: i64,
) -> Result<(Album, Vec<Image>), sqlx::Error> {
    // Get the album
    let album_row = sqlx::query!(
        r#"
        SELECT id, name, description, date, num_images, camera_model, lens_model, aperture
        FROM albums
        WHERE id = ?
        "#,
        album_id
    )
    .fetch_one(pool)
    .await?;

    let album = Album {
        id: album_row.id,
        name: album_row.name,
        description: album_row.description,
        date: album_row.date,
        num_images: album_row.num_images.unwrap_or(0) as i32,
        camera_model: album_row.camera_model,
        lens_model: album_row.lens_model,
        aperture: album_row.aperture,
    };

    // Get all images for this album
    let image_rows = sqlx::query!(
        r#"
        SELECT 
            id, album_id, filename, camera_make, camera_model, 
            lens_model, iso, aperture, shutter_speed, focal_length,
            light_source, date_created, file_size
        FROM images
        WHERE album_id = ?
        ORDER BY id ASC
        "#,
        album_id
    )
    .fetch_all(pool)
    .await?;

    let images: Vec<Image> = image_rows
        .into_iter()
        .map(|row| Image {
            id: row.id,
            album_id: row.album_id,
            filename: row.filename,
            camera_make: row.camera_make,
            camera_model: row.camera_model,
            lens_model: row.lens_model,
            iso: row.iso,
            aperture: row.aperture,
            shutter_speed: row.shutter_speed,
            focal_length: row.focal_length,
            light_source: row.light_source,
            date_created: row.date_created,
            file_size: row.file_size.unwrap_or(0),
        })
        .collect();

    Ok((album, images))
}
