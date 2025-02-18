use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use minijinja::context;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::Arc, time::Instant};
use tower_cookies::Cookies;

pub struct ProcessedImage {
    pub optimized: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub original_size: usize,
}

use crate::{
    auth::middleware::require_auth,
    db::{self, create_album, update_album_metadata},
    types::{AppState, CreateAlbumRequest},
    utils::{
        create_album_directory, delete_album_directory, process_and_save_images, ImageQuality,
    },
};

pub async fn admin_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    require_auth(cookies, State(state.clone())).await?;

    // Get site stats
    let (album_count, image_count, total_storage) =
        db::get_site_stats(&state.pool).await.unwrap_or((0, 0, 0));

    // Get albums with oldest image and size
    let albums = db::get_albums_with_oldest_image(&state.pool)
        .await
        .unwrap_or_default();

    let reloader_guard = state.reloader.lock().await;
    let env = reloader_guard.acquire_env().unwrap();
    let tmpl = env.get_template("admin.html").unwrap();
    let rendered = tmpl
        .render(context! {
            album_count => album_count,
            image_count => image_count,
            total_storage => (total_storage as f64 / 1024.0 / 1024.0).round(), // Convert to MB
            albums => albums
        })
        .unwrap();
    Ok(Html(rendered))
}

pub async fn create_album_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    mut multipart: Multipart,
) -> Response {
    let start_total = Instant::now();

    // Require authentication.
    if let Err(redirect) = require_auth(cookies.clone(), State(state.clone())).await {
        return redirect.into_response();
    }

    // ===== Multipart Extraction =====
    let start_multipart = Instant::now();
    let mut album_data: Option<CreateAlbumRequest> = None;
    let mut image_data: Vec<(String, Vec<u8>)> = Vec::new();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "album" => {
                if let Ok(bytes) = field.bytes().await {
                    if let Ok(data) = serde_json::from_slice(&bytes) {
                        album_data = Some(data);
                    } else {
                        return (StatusCode::BAD_REQUEST, "Invalid album data format")
                            .into_response();
                    }
                }
            }
            "images" => {
                let filename = field
                    .file_name()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown.jpg".to_string());

                let mut image_bytes = Vec::new();
                while let Ok(Some(chunk)) = field.chunk().await {
                    image_bytes.extend_from_slice(&chunk);
                }

                if !image_bytes.is_empty() {
                    image_data.push((filename, image_bytes));
                }
            }
            _ => continue,
        }
    }
    let multipart_duration = start_multipart.elapsed();
    println!("Multipart extraction took: {:?}", multipart_duration);

    let album_data = match album_data {
        Some(data) => data,
        None => return (StatusCode::BAD_REQUEST, "Missing album data").into_response(),
    };

    // ===== Album Creation & Directory Setup =====
    let start_album_creation = Instant::now();
    let album_id = match create_album(&state.pool, &album_data).await {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create album").into_response()
        }
    };

    if let Err(_) = create_album_directory(album_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create album directory",
        )
            .into_response();
    }
    let album_creation_duration = start_album_creation.elapsed();
    println!(
        "Album creation and directory setup took: {:?}",
        album_creation_duration
    );

    // ===== Image Processing (Concurrent) =====
    let start_image_processing = Instant::now();
    let processed_images = match process_and_save_images(state.clone(), album_id, image_data).await
    {
        Ok(count) => count,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Update album metadata
    if let Err(e) = update_album_metadata(&state.pool, album_id).await {
        eprintln!("Failed to update album metadata: {:?}", e);
    }

    let image_processing_duration = start_image_processing.elapsed();
    println!(
        "Image processing loop took: {:?}",
        image_processing_duration
    );

    let total_duration = start_total.elapsed();
    println!("Total handler execution time: {:?}", total_duration);

    Json(json!({
        "status": "success",
        "album_id": album_id,
        "images_processed": processed_images,
        "timings": {
            "multipart_extraction": format!("{:?}", multipart_duration),
            "album_creation": format!("{:?}", album_creation_duration),
            "image_processing": format!("{:?}", image_processing_duration),
            "total": format!("{:?}", total_duration)
        }
    }))
    .into_response()
}

pub async fn delete_album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Require authentication
    if let Err(_) = require_auth(cookies, State(state.clone())).await {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    // Delete from database
    if let Err(e) = db::delete_album(&state.pool, album_id).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }

    // Delete files
    if let Err(e) = delete_album_directory(album_id).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }

    Ok(Json(json!({
        "status": "success",
        "message": format!("Album {} deleted successfully", album_id)
    })))
}

pub async fn delete_image_handler(
    Path(image_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Response {
    if let Err(_) = require_auth(cookies, State(state.clone())).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Retrieve image information from the database
    let image = match db::get_image(&state.pool, image_id).await {
        Ok(Some(image)) => image,
        Ok(None) => return (StatusCode::NOT_FOUND, "Image not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // Delete from database
    if let Err(e) = db::delete_image(&state.pool, image_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    // Delete files from filesystem
    let base_path = PathBuf::from("uploads").join(image.album_id.to_string());
    for quality in [
        ImageQuality::Full,
        ImageQuality::Optimized,
        ImageQuality::Thumbnail,
    ] {
        let file_path = base_path.join(quality.as_str()).join(&image.filename);
        if let Err(e) = fs::remove_file(file_path) {
            eprintln!("Failed to delete image file: {}", e);
        }
    }

    Json(json!({"status": "success"})).into_response()
}

pub async fn update_album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    mut multipart: Multipart,
) -> Response {
    let start_total = Instant::now();

    // Authentication check
    if let Err(redirect) = require_auth(cookies, State(state.clone())).await {
        return redirect.into_response();
    }

    // Multipart extraction
    let mut album_data: Option<CreateAlbumRequest> = None;
    let mut new_images = Vec::new();
    let mut deleted_image_ids = Vec::new();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "album" => {
                if let Ok(bytes) = field.bytes().await {
                    if let Ok(data) = serde_json::from_slice(&bytes) {
                        album_data = Some(data);
                    } else {
                        return (StatusCode::BAD_REQUEST, "Invalid album data format")
                            .into_response();
                    }
                }
            }
            "new_images" => {
                let filename = field
                    .file_name()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown.jpg".to_string());

                let mut image_bytes = Vec::new();
                while let Ok(Some(chunk)) = field.chunk().await {
                    image_bytes.extend_from_slice(&chunk);
                }

                if !image_bytes.is_empty() {
                    new_images.push((filename, image_bytes));
                }
            }
            "deleted_images" => {
                if let Ok(bytes) = field.bytes().await {
                    let ids_str = String::from_utf8_lossy(&bytes);
                    deleted_image_ids = ids_str
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

    // Update album metadata if provided
    if let Some(album_data) = &album_data {
        if let Err(e) = db::update_album_details(
            &state.pool,
            album_id,
            &album_data.name,
            &album_data.description,
            &album_data.date,
        )
        .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    // Delete requested images
    let mut deleted_count = 0;

    for image_id in deleted_image_ids {
        match db::get_image(&state.pool, image_id).await {
            Ok(Some(image)) => {
                // Delete from database
                if let Err(e) = db::delete_image(&state.pool, image_id).await {
                    eprintln!("Failed to delete image {}: {}", image_id, e);
                    continue;
                }

                // Delete files
                let base_path = PathBuf::from("uploads").join(image.album_id.to_string());
                for quality in [
                    ImageQuality::Full,
                    ImageQuality::Optimized,
                    ImageQuality::Thumbnail,
                ] {
                    let file_path = base_path.join(quality.as_str()).join(&image.filename);
                    if let Err(e) = fs::remove_file(file_path) {
                        eprintln!("Failed to delete file: {}", e);
                    }
                }
                deleted_count += 1;
            }
            Ok(None) => {
                println!("Image ID {} not found in database", image_id);
            }
            Err(e) => {
                eprintln!("Error fetching image {}: {}", image_id, e);
            }
        }
    }

    // Process new images
    let processed_images = match process_and_save_images(state.clone(), album_id, new_images).await
    {
        Ok(count) => count,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Update album statistics
    if let Err(e) = db::update_album_metadata(&state.pool, album_id).await {
        eprintln!("Failed to update album metadata: {}", e);
    }

    Json(json!({
        "status": "success",
        "album_id": album_id,
        "updated_fields": album_data.is_some(),
        "deleted_images": deleted_count,
        "new_images_added": processed_images,
        "processing_time": format!("{:?}", start_total.elapsed())
    }))
    .into_response()
}

pub async fn get_album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Response {
    // Require authentication
    if let Err(redirect) = require_auth(cookies, State(state.clone())).await {
        return redirect.into_response();
    }

    match db::get_album_with_images(&state.pool, album_id).await {
        Ok((album, images)) => Json(json!({
            "status": "success",
            "album": album,
            "images": images.iter().map(|img| {
                json!({
                    "id": img.id,
                    "name": img.filename,
                    "thumbnail": format!("/uploads/{}/thumbnail/{}", album_id, img.filename),
                    "size": (img.file_size as f64 / 1024.0 / 1024.0).round()
                })
            }).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
