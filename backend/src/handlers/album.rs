use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use bytes::BytesMut;
use log::{debug, error, info, warn};
use std::error::Error;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::{
    fs::{self, File},
    time::Duration,
};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{
    db::album::{
        add_images, create_album, delete_album_images, delete_album_record, delete_images, get_album_by_id, get_albums, update_album
    },
    handlers::auth::verify_admin_request,
    models::{AppState, AlbumFields},
};

async fn process_field_stream(
    field: &mut axum::extract::multipart::Field<'_>,
) -> Result<Vec<u8>, String> {
    let mut buffer = BytesMut::new();
    let mut total_bytes = 0;
    let mut chunk_count = 0;
    const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB limit
    const CHUNK_TIMEOUT: Duration = Duration::from_secs(30);

    debug!(
        "Starting to process field stream for file: {:?}, content-type: {:?}",
        field.file_name(),
        field.content_type(),
    );

    loop {
        // Add timeout to chunk reading
        match timeout(CHUNK_TIMEOUT, field.chunk()).await {
            Ok(chunk_result) => {
                match chunk_result {
                    Ok(Some(chunk)) => {
                        chunk_count += 1;
                        let chunk_size = chunk.len();

                        debug!(
                            "Processing chunk {} - Size: {} bytes, is_empty: {}",
                            chunk_count,
                            chunk_size,
                            chunk.is_empty()
                        );

                        total_bytes += chunk_size;
                        if total_bytes > MAX_FILE_SIZE {
                            error!(
                                "File size exceeds maximum allowed size of {} bytes",
                                MAX_FILE_SIZE
                            );
                            return Err(format!(
                                "File size exceeds maximum allowed size of {} MB",
                                MAX_FILE_SIZE / 1024 / 1024
                            ));
                        }

                        // Process chunk in smaller segments if it's large
                        let mut offset = 0;
                        while offset < chunk.len() {
                            let end = std::cmp::min(offset + 65536, chunk.len());
                            buffer.extend_from_slice(&chunk[offset..end]);

                            if end - offset == 65536 {
                                debug!(
                                    "Added 64KB segment of chunk {}. Total bytes: {}",
                                    chunk_count, total_bytes
                                );
                            }

                            offset = end;
                        }

                        debug!("Successfully processed full chunk {}", chunk_count);
                    }
                    Ok(None) => {
                        debug!(
                            "Finished reading file stream. Total chunks: {}, Total bytes: {}",
                            chunk_count, total_bytes
                        );
                        break;
                    }
                    Err(e) => {
                        error!(
                            "Error reading chunk {}: {:?}. Error type: {}. Total bytes: {}",
                            chunk_count,
                            e,
                            std::any::type_name_of_val(&e),
                            total_bytes
                        );

                        if let Some(source) = e.source() {
                            error!("Underlying error: {:?}", source);
                        }

                        // Try to recover if we have partial data
                        if total_bytes > 0 {
                            warn!(
                                "Stream error occurred but we have partial data ({} bytes). Attempting to continue...",
                                total_bytes
                            );
                            break;
                        }

                        return Err(format!(
                            "Failed to read file chunk {}. Error: {}. Total bytes: {}",
                            chunk_count, e, total_bytes
                        ));
                    }
                }
            }
            Err(_) => {
                error!(
                    "Timeout reading chunk after {} seconds",
                    CHUNK_TIMEOUT.as_secs()
                );
                // Try to recover if we have partial data
                if total_bytes > 0 {
                    warn!(
                        "Timeout occurred but we have partial data ({} bytes). Attempting to continue...",
                        total_bytes
                    );
                    break;
                }
                return Err("Timeout reading file chunk".to_string());
            }
        }

        // Add periodic progress logging
        if chunk_count % 5 == 0 {
            debug!(
                "Upload progress: {} chunks, {} bytes processed",
                chunk_count, total_bytes
            );
        }
    }

    Ok(buffer.to_vec())
}

pub async fn create_album_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    debug!("Starting create_album_handler");

    // Log all headers for debugging
    for (name, value) in headers.iter() {
        debug!("Header: {} = {:?}", name, value);
    }

    // Check specific important headers
    if let Some(content_type) = headers.get("content-type") {
        debug!("Content-Type details: {:?}", content_type);
        if let Ok(ct_str) = content_type.to_str() {
            if let Some(boundary) = ct_str.split("boundary=").nth(1) {
                debug!("Multipart boundary: {}", boundary);
            }
        }
    }

    debug!("Content-Length header: {:?}", headers.get("content-length"));
    debug!(
        "Transfer-Encoding header: {:?}",
        headers.get("transfer-encoding")
    );

    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }

    let mut fields = AlbumFields::default();
    let mut uploaded_files: Vec<String> = Vec::new();
    let mut field_count = 0;

    loop {
        match multipart.next_field().await {
            Ok(Some(mut field)) => {
                field_count += 1;
                let field_name = field.name().unwrap_or("").to_string();
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());

                debug!(
                    "Processing field #{}: name='{}', filename={:?}, content_type={:?}",
                    field_count, field_name, file_name, content_type
                );

                match field_name.as_str() {
                    "name" | "description" | "date" => {
                        let text_result = field.text().await;
                        match text_result.as_ref() {
                            Ok(text) => debug!("Read text field '{}': {}", field_name, text),
                            Err(e) => error!("Failed to read text field '{}': {:?}", field_name, e),
                        }

                        match field_name.as_str() {
                            "name" => fields.name = text_result.as_ref().ok().cloned(),
                            "description" => {
                                fields.description = text_result.as_ref().ok().cloned()
                            }
                            "date" => fields.date = text_result.as_ref().ok().cloned(),
                            _ => unreachable!(),
                        }

                        if text_result.is_err() {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({
                                    "error": format!("Failed to read {} field", field_name),
                                    "details": format!("{}", text_result.unwrap_err())
                                })),
                            )
                                .into_response();
                        }
                    }
                    "images" => {
                        let file_name = field.file_name().unwrap_or("unknown").to_string();
                        info!("Processing image file: {}", file_name);

                        match process_field_stream(&mut field).await {
                            Ok(file_bytes) => {
                                info!(
                                    "Successfully read {} bytes for file {}",
                                    file_bytes.len(),
                                    file_name
                                );
                                match save_file_to_album_folder(
                                    &fields.name.clone().unwrap_or_default(),
                                    file_bytes,
                                ) {
                                    Ok(file_path) => {
                                        info!("Successfully saved file to {}", file_path);
                                        uploaded_files.push(file_path);
                                    }
                                    Err((status, body)) => {
                                        error!("Failed to save file {}: {:?}", file_name, body);
                                        return (status, body).into_response();
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to process file {}: {}", file_name, e);
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({
                                        "error": "Failed to process file upload",
                                        "file": file_name,
                                        "details": e
                                    })),
                                )
                                    .into_response();
                            }
                        }
                    }
                    _ => {
                        warn!("Ignoring unexpected field: {}", field_name);
                    }
                }
            }
            Ok(None) => {
                debug!(
                    "No more fields to process. Total fields processed: {}",
                    field_count
                );
                break;
            }
            Err(e) => {
                error!("Failed to get next field: {:?}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Failed to process form data",
                        "details": format!("{}", e)
                    })),
                )
                    .into_response();
            }
        }
    }

    debug!("Finished processing all fields. Creating album in database...");

    // Create album in database
    let album_name = fields.name.clone().unwrap_or_default();
    let album_desc = fields.description.clone().unwrap_or_default();
    let album_date = fields.date.clone().unwrap_or_default();
    let num_imgs = uploaded_files.len() as i32;

    match create_album(&state.pool, &album_name, &album_desc, &album_date, num_imgs).await {
        Ok(new_album) => {
            match add_images(&state.pool, new_album.id.clone(), album_name.clone(), &uploaded_files).await { // Pass album_name
                Ok(_) => {
                    info!(
                        "Successfully created album {} with {} images",
                        new_album.id, num_imgs
                    );
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "message": "Album created successfully",
                            "album": {
                                "id": new_album.id,
                                "name": new_album.name,
                                "description": new_album.description,
                                "date": new_album.date,
                                "number_of_images": new_album.number_of_images
                            },
                            "images": uploaded_files
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("Error inserting image records: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Failed to store images metadata",
                            "details": format!("{}", e)
                        })),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Error inserting album: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create album",
                    "details": format!("{}", e)
                })),
            )
                .into_response()
        }
    }
}

fn save_file_to_album_folder(
    album_name: &str,
    file_bytes: Vec<u8>,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let safe_album_name = album_name.replace("/", "_");
    let dir_path = format!("uploads/{}", safe_album_name);

    if let Err(e) = std::fs::create_dir_all(&dir_path) {
        error!("Failed to create album directory {}: {:?}", dir_path, e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create album directory",
                "details": format!("{}", e)
            })),
        ));
    }

    let file_name = format!("{}.jpg", Uuid::new_v4());
    let file_path = PathBuf::from(format!("{}/{}", dir_path, file_name));
    info!("Saving file to {:?}", file_path);

    let file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to create file: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create file",
                    "details": format!("{}", e)
                })),
            ));
        }
    };

    let mut writer = BufWriter::new(file);
    if let Err(e) = writer.write_all(&file_bytes) {
        error!("Failed to write file: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to write file",
                "details": format!("{}", e)
            })),
        ));
    }

    Ok(format!("{}/{}", safe_album_name, file_name))
}

/// Update an album by ID
pub async fn update_album_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(album_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Verify admin
    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }

    let mut fields = AlbumFields::default();
    let mut uploaded_files: Vec<String> = Vec::new();
    let mut images_to_delete: Vec<String> = Vec::new();

    // Get existing album first to check for name change
    let existing_album = match get_album_by_id(&state.pool, album_id.clone()).await {
        Ok((album, _)) => album,
        Err(e) => {
            error!("Error fetching album to update: {:?}", e);
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Album not found",
                    "details": format!("{}", e)
                })),
            )
                .into_response();
        }
    };

    // Process multipart form
    while let Ok(Some(mut field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "name" => fields.name = field.text().await.ok(),
            "description" => fields.description = field.text().await.ok(),
            "date" => fields.date = field.text().await.ok(),
            "images" => {
                if let Ok(file_bytes) = process_field_stream(&mut field).await {
                    // Use existing album name here - we'll move files later if name changes
                    match save_file_to_album_folder(&existing_album.name, file_bytes) {
                        Ok(file_path) => uploaded_files.push(file_path),
                        Err((status, body)) => return (status, body).into_response(),
                    }
                }
            }
            "imagesToDelete" => {
                if let Ok(ids_str) = field.text().await {
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&ids_str) {
                        images_to_delete.extend(ids);
                    }
                }
            }
            _ => continue,
        }
    }

    // Calculate new image count
    let new_image_count = existing_album.number_of_images + 
                         uploaded_files.len() as i32 - 
                         images_to_delete.len() as i32;

    // 1. First, delete marked images to release file handles
    if !images_to_delete.is_empty() {
        if let Err(e) = delete_images(&state.pool, &album_id, &images_to_delete).await {
            error!("Error deleting images: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to delete images",
                    "details": format!("{}", e)
                })),
            )
                .into_response();
        }
    }

    // 2. Handle album rename if needed
    let final_album_name = fields
        .name
        .clone()
        .unwrap_or_else(|| existing_album.name.clone());

    if final_album_name != existing_album.name {
        let old_path = format!("uploads/{}", existing_album.name.replace("/", "_"));
        let new_path = format!("uploads/{}", final_album_name.replace("/", "_"));

        // Add small delay to ensure file handles are released
        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Err(e) = fs::rename(&old_path, &new_path) {
            error!("Error renaming album directory: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to rename album directory",
                    "details": format!("{}", e)
                })),
            )
                .into_response();
        }

        // Update file paths for newly uploaded files to reflect new album name
        uploaded_files = uploaded_files
            .into_iter()
            .map(|path| {
                path.replace(
                    &existing_album.name.replace("/", "_"),
                    &final_album_name.replace("/", "_"),
                )
            })
            .collect();
    }

    // 3. Update album in database
    match update_album(
        &state.pool,
        &album_id,
        &final_album_name,
        &fields.description.unwrap_or(existing_album.description.clone()),
        &fields
            .date
            .unwrap_or(existing_album.date.clone().unwrap_or_default()),
        new_image_count,
    )
    .await
    {
        Ok(updated_album) => {
            // 4. Add new images to database if any
            if !uploaded_files.is_empty() {
                if let Err(e) = add_images(&state.pool, album_id.clone(), final_album_name, &uploaded_files).await {
                    error!("Error adding new images: {:?}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Failed to add new images",
                            "details": format!("{}", e)
                        })),
                    )
                        .into_response();
                }
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Album updated successfully",
                    "album": updated_album,
                    "new_images": uploaded_files,
                    "deleted_images": images_to_delete
                })),
            )
                .into_response()
        }
        Err(e) => {
            error!("Error updating album: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to update album",
                    "details": format!("{}", e)
                })),
            )
                .into_response()
        }
    }
}


pub async fn delete_album_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(album_id): Path<String>,
) -> impl IntoResponse {
    // Verify admin
    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }

    // Get album details before deletion
    match get_album_by_id(&state.pool, album_id.clone()).await {
        Ok((album, _)) => {
            // Delete images from filesystem
            let album_dir = format!("uploads/{}", album.name.replace("/", "_"));
            if let Err(e) = fs::remove_dir_all(&album_dir) {
                error!("Error removing album directory: {:?}", e);
                // Continue with database cleanup even if file deletion fails
            }

            // Delete images from database
            if let Err(e) = delete_album_images(&state.pool, &album_id).await {
                error!("Error deleting album images from database: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to delete album images",
                        "details": format!("{}", e)
                    })),
                )
                    .into_response();
            }

            // Delete album record
            match delete_album_record(&state.pool, &album_id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "message": "Album deleted successfully",
                        "album_id": album_id
                    })),
                )
                    .into_response(),
                Err(e) => {
                    error!("Error deleting album record: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Failed to delete album record",
                            "details": format!("{}", e)
                        })),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Error fetching album to delete: {:?}", e);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Album not found",
                    "details": format!("{}", e)
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/albums
pub async fn get_albums_handler(State(state): State<AppState>) -> impl IntoResponse {
    info!("Received request to fetch all albums");
    match get_albums(&state.pool).await {
        Ok(albums) => {
            info!("Successfully fetched {} albums", albums.len());
            (StatusCode::OK, Json(albums)).into_response()
        }
        Err(e) => {
            error!("Error fetching albums: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch albums"})),
            )
                .into_response()
        }
    }
}

/// GET /api/albums/:id
pub async fn get_album_handler(
    State(state): State<AppState>,
    Path(album_id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Received request to fetch album with ID: {}", album_id);

    match get_album_by_id(&state.pool, album_id.to_string()).await {
        Ok((album, images)) => {
            info!(
                "Successfully fetched album: {:?} with {} images",
                album,
                images.len()
            );
            let response = serde_json::json!({
                "album": album,
                "images": images
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Error fetching album {}: {:?}", album_id, e);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Album not found"})),
            )
                .into_response()
        }
    }
}
