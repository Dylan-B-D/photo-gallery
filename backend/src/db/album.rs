use std::{fs, path::Path};

use log::error;
use uuid::Uuid;
use sqlx::{SqlitePool, Row};
use crate::{models::{Album, AlbumImage}, utils::exif_utils::parse_exif_metadata};

use super::metadata::add_image_metadata;

/// Insert a new album row, returning the created `Album` data
pub async fn create_album(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    date: &str,
    number_of_images: i32,
) -> Result<Album, sqlx::Error> {
    let new_id = Uuid::new_v4().to_string();

    let number_of_images_i64 = number_of_images as i64;  // Convert i32 to i64 for SQLite

    sqlx::query!(
        r#"
        INSERT INTO albums (id, name, description, date, number_of_images)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        new_id,
        name,
        description,
        date,
        number_of_images_i64
    )
    .execute(pool)
    .await?;

    let row = sqlx::query!(
        r#"
        SELECT id, name, description, date, number_of_images
        FROM albums
        WHERE id = ?1
        "#,
        new_id
    )
    .fetch_one(pool)
    .await?;

    Ok(Album {
        id: row.id.expect("Album ID should exist"),
        name: row.name,
        description: row.description.expect("Album description should exist"),
        date: row.date,  // This is already Option<String>
        number_of_images: row.number_of_images as i32,  // Convert i64 to i32
    })
}

/// Insert multiple images for a given album
pub async fn add_images(
    pool: &SqlitePool,
    album_id: String,
    album_name: String,
    file_paths: &[String],
) -> Result<Vec<AlbumImage>, sqlx::Error> {
    let mut images = Vec::new();

    for file_path in file_paths {
        let image_id = uuid::Uuid::new_v4().to_string();

        // Extract the file name
        let file_name = match Path::new(file_path).file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => file_path.clone(),
        };

        // 1. Insert the image record
        sqlx::query!(
            r#"
            INSERT INTO images (id, album_id, file_name)
            VALUES (?1, ?2, ?3)
            "#,
            image_id,
            album_id,
            file_name
        )
        .execute(pool)
        .await?;

        // 2. Construct the full path using album_name
        let full_path = Path::new("uploads")
            .join(&album_name) // Use album_name instead of album_id
            .join(&file_name)
            .to_string_lossy()
            .to_string();

        // 3. Attempt to parse EXIF
        match parse_exif_metadata(Path::new(&full_path)) {
            Ok(mut meta) => {
                meta.image_id = image_id.clone(); // fill in the new ID
                if let Err(e) = add_image_metadata(pool, &meta).await {
                    log::warn!(
                        "Failed to store EXIF metadata for {}: {:?}",
                        full_path,
                        e
                    );
                }
            }
            Err(e) => {
                log::warn!("No valid EXIF metadata found for {}: {:?}", full_path, e);
            }
        }

        images.push(AlbumImage {
            id: image_id.clone(),
            album_id: album_id.clone(),
            file_name,
        });
    }

    Ok(images)
}



/// Fetch all albums (without images).
pub async fn get_albums(pool: &SqlitePool) -> Result<Vec<Album>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, description, date, number_of_images
        FROM albums
        ORDER BY date DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    let albums = rows
        .into_iter()
        .map(|row| Album {
            id: row.id.expect("Album ID should exist"),
            name: row.name,
            description: row.description.expect("Album description should exist"),
            date: row.date,
            number_of_images: row.number_of_images as i32,
        })
        .collect();

    Ok(albums)
}

/// Fetch one album + its images.
pub async fn get_album_by_id(
    pool: &SqlitePool,
    album_id: String,
) -> Result<(Album, Vec<AlbumImage>), sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, name, description, date, number_of_images
        FROM albums
        WHERE id = ?1
        "#,
        album_id
    )
    .fetch_one(pool)
    .await?;

    let album = Album {
        id: row.id.expect("Album ID should exist"),
        name: row.name,
        description: row.description.expect("Album description should exist"),
        date: row.date,
        number_of_images: row.number_of_images as i32,
    };

    let image_rows = sqlx::query!(
        r#"
        SELECT id, album_id, file_name
        FROM images
        WHERE album_id = ?1
        "#,
        album_id
    )
    .fetch_all(pool)
    .await?;

    let images = image_rows
        .into_iter()
        .map(|row| AlbumImage {
            id: row.id.expect("Image ID should exist"),
            album_id: row.album_id,
            file_name: row.file_name,
        })
        .collect();

    Ok((album, images))
}

pub async fn update_album(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: &str,
    date: &str,
    number_of_images: i32,
) -> Result<Album, sqlx::Error> {
    let number_of_images_i64 = number_of_images as i64;

    sqlx::query!(
        r#"
        UPDATE albums 
        SET name = ?1, description = ?2, date = ?3, number_of_images = ?4
        WHERE id = ?5
        "#,
        name,
        description,
        date,
        number_of_images_i64,
        id
    )
    .execute(pool)
    .await?;

    let row = sqlx::query!(
        r#"
        SELECT id, name, description, date, number_of_images
        FROM albums
        WHERE id = ?1
        "#,
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(Album {
        id: row.id.expect("Album ID should exist"),
        name: row.name,
        description: row.description.expect("Album description should exist"),
        date: row.date,
        number_of_images: row.number_of_images as i32,
    })
}

pub async fn delete_album_images(
    pool: &SqlitePool,
    album_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM images
        WHERE album_id = ?1
        "#,
        album_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_images(
    pool: &SqlitePool,
    album_id: &str,
    image_ids: &[String],
) -> Result<(), sqlx::Error> {
    if image_ids.is_empty() {
        return Ok(()); // Nothing to delete
    }

    // 1. Retrieve album_name from album_id
    let album_row = sqlx::query!(
        r#"
        SELECT name
        FROM albums
        WHERE id = ?1
        "#,
        album_id
    )
    .fetch_one(pool)
    .await?;

    let album_name = album_row.name.replace("/", "_"); // Sanitize if necessary

    // 2. Retrieve file_names for the given image_ids
    // Dynamically build the query with the correct number of placeholders
    let mut query_builder = String::from("SELECT file_name FROM images WHERE album_id = ?1 AND id IN (");
    let placeholders: Vec<String> = image_ids.iter().map(|_| "?".to_string()).collect();
    query_builder.push_str(&placeholders.join(", "));
    query_builder.push(')');

    let mut query = sqlx::query(&query_builder).bind(album_id);
    for image_id in image_ids {
        query = query.bind(image_id);
    }

    let image_rows = query.fetch_all(pool).await?;

    let file_names: Vec<String> = image_rows
        .into_iter()
        .map(|row| row.get("file_name"))
        .collect();

    // 3. Delete image records from the database
    let mut delete_query = String::from("DELETE FROM images WHERE album_id = ?1 AND id IN (");
    delete_query.push_str(&placeholders.join(", "));
    delete_query.push(')');

    let mut delete_query = sqlx::query(&delete_query).bind(album_id);
    for image_id in image_ids {
        delete_query = delete_query.bind(image_id);
    }
    delete_query.execute(pool).await?;

    // 4. Delete image files from the filesystem
    for file_name in file_names {
        let image_path = format!("uploads/{}/{}", album_name, file_name);
        if let Err(e) = fs::remove_file(&image_path) {
            error!("Failed to delete image file: {:?}, error: {:?}", image_path, e);
        }
    }

    Ok(())
}


pub async fn delete_album_record(
    pool: &SqlitePool,
    album_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM albums
        WHERE id = ?1
        "#,
        album_id
    )
    .execute(pool)
    .await?;

    Ok(())
}