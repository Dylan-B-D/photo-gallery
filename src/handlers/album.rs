use crate::{db::get_album_with_images, types::AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use sqlx::SqlitePool;
use std::sync::Arc;

pub async fn album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match get_album_with_images(&state.pool, album_id).await {
        Ok((album, images)) => {
            // Fetch previous and next valid album IDs
            let (prev_album, next_album) = get_adjacent_albums(&state.pool, album_id).await;

            let reloader_guard = state.reloader.lock().await;
            let env = reloader_guard.acquire_env().unwrap();
            let tmpl = env.get_template("album.html").unwrap();

            Html(
                tmpl.render(context! {
                    album => album,
                    images => images,
                    prev_album => prev_album,
                    next_album => next_album,
                })
                .unwrap(),
            )
            .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn get_adjacent_albums(pool: &SqlitePool, album_id: i64) -> (Option<i64>, Option<i64>) {
    let prev_album = sqlx::query_scalar!(
        "SELECT id FROM albums WHERE id < ? ORDER BY id DESC LIMIT 1",
        album_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let next_album = sqlx::query_scalar!(
        "SELECT id FROM albums WHERE id > ? ORDER BY id ASC LIMIT 1",
        album_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    (prev_album, next_album)
}
