use crate::types::{Album, CreateAlbumRequest, Image};
use sqlx::Row;
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

pub struct CreateImageParams<'a> {
    pub album_id: i64,
    pub filename: &'a str,
    pub file_size: i64,
    pub camera_make: &'a str,
    pub camera_model: &'a str,
    pub lens_model: &'a str,
    pub iso: &'a str,
    pub aperture: &'a str,
    pub shutter_speed: &'a str,
    pub focal_length: &'a str,
    pub light_source: &'a str,
    pub date_created: &'a str,
}

pub async fn create_image(
    pool: &SqlitePool,
    params: CreateImageParams<'_>,
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
        params.album_id,
        params.filename,
        params.file_size,
        params.camera_make,
        params.camera_model,
        params.lens_model,
        params.iso,
        params.aperture,
        params.shutter_speed,
        params.focal_length,
        params.light_source,
        params.date_created,
    )
    .execute(pool)
    .await?;

    // Update the number of images in the album
    sqlx::query!(
        "UPDATE albums SET num_images = num_images + 1 WHERE id = ?",
        params.album_id
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_album_metadata(pool: &SqlitePool, album_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE albums 
        SET 
            num_images = (SELECT COUNT(*) FROM images WHERE album_id = ?),
            camera_model = (
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
        album_id, // for num_images
        album_id, // for camera_model
        album_id, // for lens_model
        album_id, // for aperture
        album_id  // for the WHERE clause
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_albums_with_oldest_image(
    pool: &SqlitePool,
) -> Result<Vec<(Album, Option<String>, i64)>, sqlx::Error> {
    let results = sqlx::query(
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
            a.cover_image_id,
            COALESCE(
                (SELECT i.filename FROM images i WHERE i.id = a.cover_image_id),
                (
                    SELECT i.filename
                    FROM images i
                    WHERE i.album_id = a.id
                    ORDER BY i.date_created ASC
                    LIMIT 1
                )
            ) AS cover_image
        FROM albums a
        ORDER BY a.date DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut albums_with_size = Vec::new();
    for row in results {
        let album_id: i64 = row.try_get("id")?;
        let album_size = get_album_size(pool, album_id).await?;

        let num_images: Option<i64> = row.try_get("num_images")?;
        let cover_image_id: Option<i64> = row.try_get("cover_image_id")?;
        let cover_image: Option<String> = row.try_get("cover_image")?;

        albums_with_size.push((
            Album {
                id: album_id,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                date: row.try_get("date")?,
                num_images: num_images.unwrap_or(0) as i32,
                camera_model: row.try_get("camera_model")?,
                lens_model: row.try_get("lens_model")?,
                aperture: row.try_get("aperture")?,
                cover_image_id,
            },
            cover_image,
            album_size,
        ));
    }

    Ok(albums_with_size)
}

pub async fn get_album_with_images(
    pool: &SqlitePool,
    album_id: i64,
) -> Result<(Album, Vec<Image>), sqlx::Error> {
    // Get the album
    let album_row = sqlx::query(
        r#"
        SELECT id, name, description, date, num_images, camera_model, lens_model, aperture, cover_image_id
        FROM albums
        WHERE id = ?
        "#,
    )
    .bind(album_id)
    .fetch_one(pool)
    .await?;

    let num_images: Option<i64> = album_row.try_get("num_images")?;
    let cover_image_id: Option<i64> = album_row.try_get("cover_image_id")?;

    let album = Album {
        id: album_row.try_get("id")?,
        name: album_row.try_get("name")?,
        description: album_row.try_get("description")?,
        date: album_row.try_get("date")?,
        num_images: num_images.unwrap_or(0) as i32,
        camera_model: album_row.try_get("camera_model")?,
        lens_model: album_row.try_get("lens_model")?,
        aperture: album_row.try_get("aperture")?,
        cover_image_id,
    };

    // Get all images for this album, ordered by date_created (oldest first)
    let image_rows = sqlx::query!(
        r#"
        SELECT 
            id, album_id, filename, camera_make, camera_model, 
            lens_model, iso, aperture, shutter_speed, focal_length,
            light_source, date_created, file_size
        FROM images
        WHERE album_id = ?
        ORDER BY date_created ASC
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

pub async fn get_site_stats(pool: &SqlitePool) -> Result<(i64, i64, i64), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct SiteStats {
        album_count: Option<i64>,
        image_count: Option<i64>,
        total_storage: Option<i64>,
    }

    let stats = sqlx::query_as!(
        SiteStats,
        r#"
        SELECT 
            (SELECT COUNT(*) FROM albums) as album_count,
            (SELECT SUM(num_images) FROM albums) as image_count,
            (SELECT SUM(file_size) FROM images) as total_storage
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok((
        stats.album_count.unwrap_or(0),
        stats.image_count.unwrap_or(0),
        stats.total_storage.unwrap_or(0),
    ))
}

pub async fn get_album_size(pool: &SqlitePool, album_id: i64) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT SUM(file_size) as total_size
        FROM images
        WHERE album_id = ?
        "#,
        album_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.total_size.unwrap_or(0))
}

pub async fn delete_album(pool: &SqlitePool, album_id: i64) -> Result<(), sqlx::Error> {
    // First delete associated images
    sqlx::query!("DELETE FROM images WHERE album_id = ?", album_id)
        .execute(pool)
        .await?;

    // Then delete the album
    sqlx::query!("DELETE FROM albums WHERE id = ?", album_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_image(pool: &SqlitePool, image_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM images WHERE id = ?", image_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_image(pool: &SqlitePool, image_id: i64) -> Result<Option<Image>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            id, album_id, filename, camera_make, camera_model, 
            lens_model, iso, aperture, shutter_speed, focal_length,
            light_source, date_created, file_size
        FROM images
        WHERE id = ?
        "#,
        image_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = result {
        Ok(Some(Image {
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
        }))
    } else {
        Ok(None)
    }
}

pub async fn update_album_details(
    pool: &SqlitePool,
    album_id: i64,
    name: &str,
    description: &Option<String>,
    date: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE albums SET name = $1, description = $2, date = $3 WHERE id = $4",
        name,
        description,
        date,
        album_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_album_cover(
    pool: &SqlitePool,
    album_id: i64,
    cover_image_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE albums SET cover_image_id = ? WHERE id = ?")
        .bind(cover_image_id)
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_cover_for_image(pool: &SqlitePool, image_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE albums SET cover_image_id = NULL WHERE cover_image_id = ?")
        .bind(image_id)
        .execute(pool)
        .await?;
    Ok(())
}
