use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use minijinja::context;
use serde_json::{json, Value};
use std::future::Future;
use std::{
    fs, io,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tower_cookies::Cookies;

use crate::{
    auth::middleware::require_auth,
    db::{self, create_album, update_album_metadata},
    types::{AppState, CreateAlbumRequest},
    upload_batch::upload_batch_configs_json,
    utils::{
        create_album_directory, delete_album_directory, process_and_save_image, ImageQuality,
        SavedImage,
    },
};

type HandlerError = Box<dyn std::error::Error + Send + Sync>;
type ImageTaskResult = Result<SavedImage, HandlerError>;

fn queue_after_permit<T, F, Fut>(
    join_set: &mut JoinSet<Result<T, HandlerError>>,
    semaphore: Arc<Semaphore>,
    work: F,
) where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, HandlerError>> + Send + 'static,
{
    join_set.spawn(async move {
        let _permit = semaphore
            .acquire_owned()
            .await
            .map_err(|e| -> HandlerError { Box::new(e) })?;
        work().await
    });
}

fn image_process_timeout() -> Duration {
    let seconds = std::env::var("IMAGE_PROCESS_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(75);

    Duration::from_secs(seconds)
}

async fn process_uploaded_image(
    state: Arc<AppState>,
    album_id: i64,
    original_filename: String,
    file_bytes: Vec<u8>,
) -> ImageTaskResult {
    let byte_count = file_bytes.len();
    let started = Instant::now();
    tracing::info!(
        album_id,
        filename = %original_filename,
        bytes = byte_count,
        "processing uploaded image"
    );

    let process_timeout = image_process_timeout();
    let result = tokio::time::timeout(
        process_timeout,
        process_and_save_image(state, album_id, original_filename.clone(), file_bytes),
    )
    .await;

    match result {
        Ok(Ok(saved)) => {
            tracing::info!(
                album_id,
                filename = %original_filename,
                elapsed_ms = started.elapsed().as_millis(),
                "finished processing uploaded image"
            );
            Ok(saved)
        }
        Ok(Err(error)) => {
            tracing::error!(
                album_id,
                filename = %original_filename,
                error = %error,
                "failed to process uploaded image"
            );
            Err(error)
        }
        Err(_) => {
            let message = format!(
                "Timed out processing '{original_filename}' after {} seconds",
                process_timeout.as_secs()
            );
            tracing::error!(album_id, filename = %original_filename, "{message}");
            Err(Box::new(io::Error::new(io::ErrorKind::TimedOut, message)))
        }
    }
}

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
            albums => albums,
            upload_batch_config_json => upload_batch_configs_json()
        })
        .unwrap();
    Ok(Html(rendered))
}

pub async fn create_album_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let start_total = Instant::now();

    // Require authentication.
    if let Err(redirect) = require_auth(cookies.clone(), State(state.clone())).await {
        return redirect.into_response();
    }

    let default_concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .ok()
        .map(|n| {
            let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
            if app_env == "production" {
                n.clamp(2, 6)
            } else {
                n.clamp(4, 8)
            }
        })
        .unwrap_or(2);

    let concurrency = std::env::var("IMAGE_PROCESS_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default_concurrency);

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut join_set: JoinSet<ImageTaskResult> = JoinSet::new();

    let start_multipart = Instant::now();
    let mut album_id: Option<i64> = None;
    let mut album_creation_duration = None;
    let mut cover_name: Option<String> = None;
    let mut cover_size: Option<usize> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read upload data: {error}"),
                )
                    .into_response()
            }
        };
        let field_name = field.name().unwrap_or("");
        match field_name {
            "album" => {
                let bytes = match field.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
                };

                let album_data: CreateAlbumRequest = match serde_json::from_slice(&bytes) {
                    Ok(data) => data,
                    Err(_) => {
                        return (StatusCode::BAD_REQUEST, "Invalid album data format")
                            .into_response()
                    }
                };

                let start_album_creation = Instant::now();
                let created_album_id = match create_album(&state.pool, &album_data).await {
                    Ok(id) => id,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create album")
                            .into_response()
                    }
                };

                if create_album_directory(created_album_id).await.is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to create album directory",
                    )
                        .into_response();
                }

                album_creation_duration = Some(start_album_creation.elapsed());
                album_id = Some(created_album_id);
            }
            "cover_name" => {
                if let Ok(bytes) = field.bytes().await {
                    let s = String::from_utf8_lossy(&bytes).trim().to_string();
                    if !s.is_empty() {
                        cover_name = Some(s);
                    }
                }
            }
            "cover_size" => {
                if let Ok(bytes) = field.bytes().await {
                    let s = String::from_utf8_lossy(&bytes).trim().to_string();
                    if let Ok(size) = s.parse::<usize>() {
                        cover_size = Some(size);
                    }
                }
            }
            "images" => {
                let current_album_id = match album_id {
                    Some(id) => id,
                    None => return (StatusCode::BAD_REQUEST, "Missing album data").into_response(),
                };

                let original_filename = field
                    .file_name()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown.jpg".to_string());

                let file_bytes = match field.bytes().await {
                    Ok(bytes) => bytes.to_vec(),
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to read '{original_filename}': {error}"),
                        )
                            .into_response()
                    }
                };
                if file_bytes.is_empty() {
                    continue;
                }

                let state = state.clone();
                queue_after_permit(&mut join_set, semaphore.clone(), move || async move {
                    process_uploaded_image(state, current_album_id, original_filename, file_bytes)
                        .await
                });
            }
            _ => {}
        }
    }

    let multipart_duration = start_multipart.elapsed();

    let album_id = match album_id {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "Missing album data").into_response(),
    };

    let start_image_processing = Instant::now();
    let mut processed_images = 0usize;
    let mut saved_images: Vec<SavedImage> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(saved)) => {
                processed_images += 1;
                saved_images.push(saved);
            }
            Ok(Err(e)) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    if let (Some(cover_name), Some(cover_size)) = (cover_name.as_deref(), cover_size) {
        if let Some(saved) = saved_images
            .iter()
            .find(|img| img.original_filename == cover_name && img.original_size == cover_size)
        {
            if let Err(e) = db::set_album_cover(&state.pool, album_id, Some(saved.id)).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        }
    }

    // Update album metadata
    if let Err(e) = update_album_metadata(&state.pool, album_id).await {
        eprintln!("Failed to update album metadata: {:?}", e);
    }

    let image_processing_duration = start_image_processing.elapsed();
    let total_duration = start_total.elapsed();

    Json(json!({
        "status": "success",
        "album_id": album_id,
        "images_processed": processed_images,
        "timings": {
            "multipart_extraction": format!("{:?}", multipart_duration),
            "album_creation": format!("{:?}", album_creation_duration.unwrap_or_default()),
            "image_processing": format!("{:?}", image_processing_duration),
            "total": format!("{:?}", total_duration)
        }
    }))
    .into_response()
}

pub async fn update_album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let start_total = Instant::now();

    // Authentication check
    if let Err(redirect) = require_auth(cookies, State(state.clone())).await {
        return redirect.into_response();
    }

    let default_concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .ok()
        .map(|n| {
            let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
            if app_env == "production" {
                n.clamp(2, 6)
            } else {
                n.clamp(4, 8)
            }
        })
        .unwrap_or(2);

    let concurrency = std::env::var("IMAGE_PROCESS_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default_concurrency);

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut join_set: JoinSet<ImageTaskResult> = JoinSet::new();

    let mut album_data: Option<CreateAlbumRequest> = None;
    let mut deleted_image_ids: Vec<i64> = Vec::new();
    let mut cover_image_id: Option<i64> = None;
    let mut cover_name: Option<String> = None;
    let mut cover_size: Option<usize> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read upload data: {error}"),
                )
                    .into_response()
            }
        };
        let field_name = field.name().unwrap_or("");
        match field_name {
            "album" => {
                if let Ok(bytes) = field.bytes().await {
                    if let Ok(data) = serde_json::from_slice::<CreateAlbumRequest>(&bytes) {
                        album_data = Some(data);
                    } else {
                        return (StatusCode::BAD_REQUEST, "Invalid album data format")
                            .into_response();
                    }
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
            "cover_image_id" => {
                if let Ok(bytes) = field.bytes().await {
                    let s = String::from_utf8_lossy(&bytes).trim().to_string();
                    if let Ok(id) = s.parse::<i64>() {
                        cover_image_id = Some(id);
                    }
                }
            }
            "cover_name" => {
                if let Ok(bytes) = field.bytes().await {
                    let s = String::from_utf8_lossy(&bytes).trim().to_string();
                    if !s.is_empty() {
                        cover_name = Some(s);
                    }
                }
            }
            "cover_size" => {
                if let Ok(bytes) = field.bytes().await {
                    let s = String::from_utf8_lossy(&bytes).trim().to_string();
                    if let Ok(size) = s.parse::<usize>() {
                        cover_size = Some(size);
                    }
                }
            }
            "new_images" => {
                let original_filename = field
                    .file_name()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown.jpg".to_string());

                let file_bytes = match field.bytes().await {
                    Ok(bytes) => bytes.to_vec(),
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to read '{original_filename}': {error}"),
                        )
                            .into_response()
                    }
                };
                if file_bytes.is_empty() {
                    continue;
                }

                let state = state.clone();
                queue_after_permit(&mut join_set, semaphore.clone(), move || async move {
                    process_uploaded_image(state, album_id, original_filename, file_bytes).await
                });
            }
            _ => {}
        }
    }

    if cover_image_id.is_some() {
        if let Err(e) = db::set_album_cover(&state.pool, album_id, cover_image_id).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
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
                if let Err(e) = db::clear_cover_for_image(&state.pool, image_id).await {
                    eprintln!(
                        "Failed to clear cover image reference for {}: {}",
                        image_id, e
                    );
                }

                // Delete from database
                if let Err(e) = db::delete_image(&state.pool, image_id).await {
                    eprintln!("Failed to delete image {}: {}", image_id, e);
                    continue;
                }

                // Delete files
                let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("uploads")
                    .join(image.album_id.to_string());
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

    let mut processed_images = 0usize;
    let mut saved_images: Vec<SavedImage> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(saved)) => {
                processed_images += 1;
                saved_images.push(saved);
            }
            Ok(Err(e)) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    if cover_image_id.is_none() {
        if let (Some(cover_name), Some(cover_size)) = (cover_name.as_deref(), cover_size) {
            if let Some(saved) = saved_images
                .iter()
                .find(|img| img.original_filename == cover_name && img.original_size == cover_size)
            {
                if let Err(e) = db::set_album_cover(&state.pool, album_id, Some(saved.id)).await {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            }
        }
    }

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

pub async fn delete_album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Require authentication
    if require_auth(cookies, State(state.clone())).await.is_err() {
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
    if require_auth(cookies, State(state.clone())).await.is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Retrieve image information from the database
    let image = match db::get_image(&state.pool, image_id).await {
        Ok(Some(image)) => image,
        Ok(None) => return (StatusCode::NOT_FOUND, "Image not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if let Err(e) = db::clear_cover_for_image(&state.pool, image_id).await {
        eprintln!(
            "Failed to clear cover image reference for {}: {}",
            image_id, e
        );
    }

    // Delete from database
    if let Err(e) = db::delete_image(&state.pool, image_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    // Delete files from filesystem
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("uploads")
        .join(image.album_id.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use minijinja::{context, path_loader, Environment};
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn admin_template_renders_upload_batch_config() {
        let mut env = Environment::new();
        let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
        env.set_loader(path_loader(&template_path));
        env.add_global("app_env", "production");
        env.add_global("asset_version", "test");

        let rendered = env
            .get_template("admin.html")
            .expect("admin template exists")
            .render(context! {
                album_count => 0,
                image_count => 0,
                total_storage => 0.0,
                albums => Vec::<(crate::types::Album, Option<String>, i64)>::new(),
                upload_batch_config_json => upload_batch_configs_json()
            })
            .expect("admin template renders");

        assert!(rendered.contains("window.pgUploadBatchConfig = {\"local\""));
        assert!(rendered.contains("\"max_count\":1"));
        assert!(rendered.contains("\"request_timeout_ms\":90000"));
    }

    #[tokio::test]
    async fn queue_after_permit_does_not_wait_before_spawning_work() {
        let semaphore = Arc::new(Semaphore::new(0));
        let mut join_set: JoinSet<Result<(), HandlerError>> = JoinSet::new();
        let work_started = Arc::new(AtomicBool::new(false));
        let work_started_in_task = work_started.clone();

        queue_after_permit(&mut join_set, semaphore.clone(), move || async move {
            work_started_in_task.store(true, Ordering::SeqCst);
            Ok(())
        });

        assert_eq!(join_set.len(), 1);
        assert!(!work_started.load(Ordering::SeqCst));

        semaphore.add_permits(1);
        let result = join_set.join_next().await.expect("queued task completes");

        assert!(result.expect("task joins").is_ok());
        assert!(work_started.load(Ordering::SeqCst));
    }
}
