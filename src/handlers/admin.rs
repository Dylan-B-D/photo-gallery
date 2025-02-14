// src/handlers/admin.rs

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use minijinja::context;
use serde_json::json;
use std::{sync::Arc, time::Instant};
use tower_cookies::Cookies;

pub struct ProcessedImage {
    pub optimized: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub original_size: usize,
}

use crate::{
    auth::middleware::require_auth,
    db::{create_album, create_image, update_album_metadata},
    types::{AppState, CreateAlbumRequest},
    utils::{
        create_album_directory, extract_exif_metadata, generate_unique_filename, process_image, save_image, ImageQuality
    },
};

pub async fn admin_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    require_auth(cookies, State(state.clone())).await?;

    let reloader_guard = state.reloader.lock().await;
    let env = reloader_guard.acquire_env().unwrap();
    let tmpl = env.get_template("admin.html").unwrap();
    let rendered = tmpl.render(context! {}).unwrap();
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
    let mut tasks = Vec::new();

    for (original_filename, data) in image_data {
        let state = state.clone();
        let album_id = album_id;
        let filename = generate_unique_filename(&original_filename);

        tasks.push(tokio::spawn(async move {
            // Extract EXIF metadata from the image
            let metadata = extract_exif_metadata(&data).unwrap_or((
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
            ));

            // Save the full-resolution image immediately
            save_image(&data, &filename, album_id, ImageQuality::Full).await?;

            // Process the image into optimized and thumbnail versions
            let processed = process_image(data).await?;

            // Save optimized and thumbnail versions concurrently
            let save_optimized = save_image(
                &processed.optimized,
                &filename,
                album_id,
                ImageQuality::Optimized,
            );
            let save_thumbnail = save_image(
                &processed.thumbnail,
                &filename,
                album_id,
                ImageQuality::Thumbnail,
            );

            tokio::try_join!(save_optimized, save_thumbnail)?;

            // Create a database entry with image metadata
            create_image(
                &state.pool,
                album_id,
                &filename,
                processed.original_size as i64,
                &metadata.0, // camera_make
                &metadata.1, // camera_model
                &metadata.2, // lens_model
                &metadata.3, // iso
                &metadata.4, // aperture
                &metadata.5, // shutter_speed
                &metadata.6, // focal_length
                &metadata.7, // light_source
                &metadata.8, // date_created
            )
            .await?;

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }));
    }

    let results = futures::future::join_all(tasks).await;
    let processed_images = results.into_iter().filter(|res| res.is_ok()).count();

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
