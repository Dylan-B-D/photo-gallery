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