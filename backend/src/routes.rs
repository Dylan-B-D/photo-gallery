use crate::config::CONFIG;
use crate::handlers::album::{
    create_album_handler, delete_album_handler, get_album_handler, get_albums_handler,
    update_album_handler,
};
use crate::handlers::auth::{login_handler, verify_handler};
use crate::handlers::metadata::{get_album_mode_metadata_handler, get_image_metadata_handler};
use crate::models::AppState;
use axum::extract::DefaultBodyLimit;
use axum::{
    http::{self, HeaderValue},
    routing::{get, post},
    Router,
};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::timeout::TimeoutLayer;

pub fn create_router(pool: Pool<Sqlite>) -> Router {
    let state = AppState {
        admin_credentials: Arc::new(CONFIG.admin_credentials.clone()),
        pool,
    };

    let allowed_origins = vec!["http://localhost:3000", "http://dylanbd.duckdns.org"];

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins.iter().map(|&origin| HeaderValue::from_str(origin).unwrap()).collect::<Vec<_>>()))
        .allow_methods([
            http::Method::POST,
            http::Method::GET,
            http::Method::PUT,
            http::Method::DELETE,
        ])
        .allow_headers([
            "Content-Type".parse().unwrap(),
            "Authorization".parse().unwrap(),
        ])
        .allow_credentials(true);

    Router::new()
        .nest_service("/uploads", ServeDir::new("uploads"))
        // Auth
        .route("/api/login", post(login_handler))
        .route("/api/verify", get(verify_handler))
        // Albums
        .route(
            "/api/albums",
            post(create_album_handler).get(get_albums_handler),
        )
        .route(
            "/api/albums/{id}",
            get(get_album_handler)
                .put(update_album_handler)
                .delete(delete_album_handler),
        )
        .route("/api/images/{id}/metadata", get(get_image_metadata_handler))
        .route(
            "/api/albums/{id}/mode-metadata",
            get(get_album_mode_metadata_handler),
        )
        .layer(cors)
        .layer(DefaultBodyLimit::max(2000 * 1024 * 1024)) // 2 GB
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .with_state(state)
}
