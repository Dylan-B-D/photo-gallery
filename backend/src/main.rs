mod config;
mod db;
mod handlers;
mod models;
mod routes;

use log::{info, warn};
use routes::create_router;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    dotenv::dotenv().ok();

    // Debug print to verify .env file loading
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        info!("DATABASE_URL from .env: {}", db_url);
    } else {
        warn!("DATABASE_URL not set in .env");
    }

    // Get database URL from environment
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!("DATABASE_URL not set in environment, using default value");
        "sqlite://photo_gallery.db".to_string()
    });

    // If DB doesn't exist, create it
    if !Sqlite::database_exists(&db_url).await? {
        info!("Creating database at {}", db_url);
        Sqlite::create_database(&db_url).await?;
    }

    let pool = SqlitePool::connect(&db_url).await?;

    // The macro approach references the migrations at compile time,
    // so "migrations" folder must be in your crate root (the same as Cargo.toml).
    sqlx::migrate!().run(&pool).await?;
    info!("Migrations applied");

    let app = create_router(pool);
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!("Server running on http://localhost:8080");

    axum::serve(listener, app).await?;
    Ok(())
}