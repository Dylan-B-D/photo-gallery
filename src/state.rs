use crate::types::AppState;
use minijinja::{path_loader, Environment};
use minijinja_autoreload::AutoReloader;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

pub const TEMPLATES_DIR: &str = "templates";

/// Initializes the application state with the reloader and other configurations.
pub fn init_state(pool: SqlitePool) -> Arc<AppState> {
    let reloader = if cfg!(debug_assertions) {
        let auto_reload_mode = env::var("AUTO_RELOAD_MODE").unwrap_or_else(|_| "0".to_string());
        Arc::new(AsyncMutex::new(AutoReloader::new(move |notifier| {
            let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEMPLATES_DIR);
            let mut env = Environment::new();
            env.set_loader(path_loader(&template_path));

            // Add a custom context processor to inject `app_env`
            env.add_global(
                "app_env",
                env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            );

            match auto_reload_mode.as_str() {
                "1" => notifier.set_fast_reload(true),
                "2" => notifier.watch_path(&template_path, true),
                _ => {}
            }

            Ok(env)
        })))
    } else {
        Arc::new(AsyncMutex::new(AutoReloader::new(move |_| {
            let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEMPLATES_DIR);
            let mut env = Environment::new();
            env.set_loader(path_loader(&template_path));

            // Add a custom context processor to inject `app_env`
            env.add_global(
                "app_env",
                env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            );
            Ok(env)
        })))
    };

    Arc::new(AppState {
        reloader: Arc::clone(&reloader),
        jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        pool,
    })
}

pub async fn init_db() -> SqlitePool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    // Run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}
