use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::AdminCredentials;

#[derive(Clone)]
pub struct AppState {
    pub admin_credentials: Arc<AdminCredentials>,
    pub pool: SqlitePool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub description: String,
    pub date: Option<String>,
    pub number_of_images: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AlbumImage {
    pub id: String,
    pub album_id: String,
    pub file_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageMetadata {
    pub image_id: String,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub iso: Option<i64>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<f64>,
    pub light_source: Option<String>,
    pub date_created: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Default)]
pub struct AlbumFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModeMetadata {
    pub camera_model: Option<String>,

    pub lens_model: Option<String>,

    pub aperture: Option<String>,
}

impl Default for ModeMetadata {
    fn default() -> Self {
        ModeMetadata {
            camera_model: None,

            lens_model: None,

            aperture: None,
        }
    }
}
