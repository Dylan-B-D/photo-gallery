use axum::routing::{delete, get, post};
use axum::Router;
use dotenv::dotenv;
use handlers::admin::admin_handler;
use handlers::album::album_handler;
use handlers::home::home_handler;
use handlers::login::{login_handler, login_post_handler, logout_handler};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;

mod auth;
mod db;
mod handlers;
mod state;
mod types;
mod utils;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    dotenv().ok();

    // Initialize the database pool
    let pool = state::init_db().await;

    // Initialize the application state
    let state = state::init_state(pool);

    // Configure rate limiting
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5) // Allow 2 requests per second
            .burst_size(10) // Allow a burst of up to 5 requests
            .use_headers() // Enable `x-ratelimit-*` headers
            .finish()
            .unwrap(),
    );

    // Create routers for static files and uploads
    let static_router = Router::new().nest_service("/static", ServeDir::new("static"));
    let uploads_router = Router::new().nest_service("/uploads", ServeDir::new("uploads"));

    // Create the main app router
    let app = Router::new()
        .route("/", get(home_handler))
        .route("/login", get(login_handler).post(login_post_handler))
        .route("/admin", get(admin_handler))
        .route("/albums/{id}", get(album_handler))
        .route("/api/albums", post(handlers::admin::create_album_handler))
        .route("/api/albums/{id}", delete(handlers::admin::delete_album_handler))
        .route("/logout", get(logout_handler))
        .layer(CookieManagerLayer::new())
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(RequestBodyLimitLayer::new(2000 * 1024 * 1024)) // Limit request body size to 2 GB
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)) // 1GB limit
        .with_state(state)
        .merge(static_router)
        .merge(uploads_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    println!("listening on http://{}", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
