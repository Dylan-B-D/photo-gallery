use axum::{
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum::extract::Multipart;
use log::{error, info, warn};
use uuid::Uuid;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::{
    handlers::auth::verify_admin_request,
    models::AppState,
    db::album::{create_album, add_images},
};
use crate::db::album::{get_albums, get_album_by_id};

/// This struct is for parsing normal (non-file) fields from the multipart data.
#[derive(Debug, Default)]
pub struct AlbumFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
}

/// Handle the creation of an album
pub async fn create_album_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // 1. Verify admin
    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }

    let mut fields = AlbumFields::default();
    let mut uploaded_files: Vec<String> = Vec::new();

    while let Some(field) = match multipart.next_field().await {
        Ok(Some(f)) => Some(f),
        Ok(None) => None,
        Err(e) => {
            error!("Multipart field error: {:?}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid form data"})),
            )
                .into_response();
        }
    } {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "name" => {
                fields.name = Some(String::from_utf8(field.bytes().await.unwrap().to_vec()).unwrap());
            }
            "description" => {
                fields.description = Some(String::from_utf8(field.bytes().await.unwrap().to_vec()).unwrap());
            }
            "date" => {
                fields.date = Some(String::from_utf8(field.bytes().await.unwrap().to_vec()).unwrap());
            }
            "images" => {
                // handle file
                let file_bytes = field.bytes().await.unwrap();
                match save_file_to_album_folder(
                    &fields.name.clone().unwrap_or_default(),
                    file_bytes,
                ) {
                    Ok(file_path) => uploaded_files.push(file_path),
                    Err((status, body)) => return (status, body).into_response(),
                }
            }
            _ => {
                warn!("Ignoring unexpected field: {}", name);
            }
        }
    }

    // 2. Persist to DB
    let album_name = fields.name.clone().unwrap_or_default();
    let album_desc = fields.description.clone().unwrap_or_default();
    let album_date = fields.date.clone().unwrap_or_default();
    let num_imgs = uploaded_files.len() as i32;

    match create_album(&state.pool, &album_name, &album_desc, &album_date, num_imgs).await {
        Ok(new_album) => {
            // Insert images rows
            let image_records = add_images(&state.pool, new_album.id.clone(), &uploaded_files).await;
            if let Err(e) = image_records {
                error!("Error inserting image records: {:?}", e);
                // In a real app, consider rolling back the album insertion if image insert fails
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to store images metadata"}))
                )
                    .into_response();
            }

            return (
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
                .into_response();
        }
        Err(e) => {
            error!("Error inserting album: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create album"}))
            )
                .into_response();
        }
    }
}

/// Save the file bytes to `uploads/<albumName>/some-unique-filename.jpg`
/// Returns the relative path or filename that was stored.
fn save_file_to_album_folder(
    album_name: &str,
    file_bytes: bytes::Bytes,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let safe_album_name = album_name.replace("/", "_"); // naive sanitization
    let dir_path = format!("uploads/{}", safe_album_name);
    if let Err(e) = std::fs::create_dir_all(&dir_path) {
        error!("Failed to create album directory {}: {:?}", dir_path, e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to create album directory"})),
        ));
    }

    let file_name = format!("{}.jpg", Uuid::new_v4());
    let file_path = PathBuf::from(format!("{}/{}", dir_path, file_name));
    info!("Saving file to {:?}", file_path);

    let mut file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to create file: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create file"})),
            ));
        }
    };

    if let Err(e) = file.write_all(&file_bytes) {
        error!("Failed to write file: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to write file"})),
        ));
    }

    // Return just the final portion: "uploads/AlbumName/xyz.jpg", for example
    Ok(format!("{}/{}", safe_album_name, file_name))
}

/// Update an album by ID (stubbed for now)
pub async fn update_album_handler(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<String>,
    // you'd parse similar data from a `Multipart` or `Json` depending on your design
) -> impl IntoResponse {
    // 1. Verify admin
    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }

    // 2. handle updates ...
    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Update album - not yet implemented"})),
    ).into_response()
}

/// Delete an album by ID (stubbed for now)
pub async fn delete_album_handler(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // 1. Verify admin
    if let Err((status, body)) = verify_admin_request(&headers) {
        return (status, body).into_response();
    }
    // 2. handle deletion ...
    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Delete album - not yet implemented"})),
    ).into_response()
}

/// GET /api/albums
pub async fn get_albums_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("Received request to fetch all albums");
    match get_albums(&state.pool).await {
        Ok(albums) => {
            info!("Successfully fetched {} albums", albums.len());
            (StatusCode::OK, Json(albums)).into_response()
        },
        Err(e) => {
            error!("Error fetching albums: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch albums"}))
            ).into_response()
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
            info!("Successfully fetched album: {:?} with {} images", album, images.len());
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
                Json(serde_json::json!({"error": "Album not found"}))
            ).into_response()
        }
    }
}

