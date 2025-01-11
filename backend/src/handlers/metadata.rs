use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use crate::{
    db::metadata::{get_album_mode_metadata, get_image_metadata},
    models::{AppState, ImageMetadata},
};
use log::{info, error};

#[derive(Serialize)]
struct ImageMetadataResponse {
    metadata: ImageMetadata,
}

#[derive(Serialize)]
struct ModeMetadataResponse {
    camera_model: Option<String>,
    lens_model: Option<String>,
    aperture: Option<String>,
}

pub async fn get_image_metadata_handler(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
) -> impl IntoResponse {
    info!("Fetching metadata for image ID: {}", image_id);

    match get_image_metadata(&state.pool, &image_id).await {
        Ok(metadata) => {
            info!("Successfully fetched metadata for image ID: {}", image_id);
            (StatusCode::OK, Json(ImageMetadataResponse { metadata })).into_response()
        }
        Err(sqlx::Error::RowNotFound) => {
            error!("Image metadata not found for image ID: {}", image_id);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Image metadata not found"})),
            )
                .into_response()
        }
        Err(e) => {
            error!("Error fetching image metadata: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch image metadata"})),
            )
                .into_response()
        }
    }
}

pub async fn get_album_mode_metadata_handler(
    State(state): State<AppState>,
    Path(album_id): Path<String>,
) -> impl IntoResponse {
    info!("Fetching mode metadata for album ID: {}", album_id);

    match get_album_mode_metadata(&state.pool, &album_id).await {
        Ok(mode_metadata) => {
            info!("Successfully fetched mode metadata for album ID: {}", album_id);
            let response = ModeMetadataResponse {
                camera_model: mode_metadata.camera_model,
                lens_model: mode_metadata.lens_model,
                aperture: mode_metadata.aperture,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(sqlx::Error::RowNotFound) => {
            error!("No metadata found for album ID: {}", album_id);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "No metadata found for this album"})),
            )
                .into_response()
        }
        Err(e) => {
            error!("Error fetching mode metadata: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch mode metadata"})),
            )
                .into_response()
        }
    }
}