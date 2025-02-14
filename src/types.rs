use minijinja_autoreload::AutoReloader;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, SqlitePool};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct AppState {
    pub reloader: Arc<AsyncMutex<AutoReloader>>,
    pub jwt_secret: String,
    pub pool: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlbumRequest {
    pub name: String,
    pub description: Option<String>,
    pub date: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub date: String,
    pub num_images: i32,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub aperture: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Image {
    pub id: i64,
    pub album_id: i64,
    pub filename: String,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub iso: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<String>,
    pub light_source: Option<String>,
    pub date_created: Option<String>,
    pub file_size: i64,
}