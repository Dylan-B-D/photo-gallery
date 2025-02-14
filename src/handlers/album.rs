use crate::{types::AppState, db::get_album_with_images};
use axum::{
    extract::{Path, State}, http::StatusCode, response::{Html, IntoResponse, Response}
};
use minijinja::context;
use std::sync::Arc;

pub async fn album_handler(
    Path(album_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match get_album_with_images(&state.pool, album_id).await {
        Ok((album, images)) => {
            let reloader_guard = state.reloader.lock().await;
            let env = reloader_guard.acquire_env().unwrap();
            let tmpl = env.get_template("album.html").unwrap();
            
            Html(tmpl.render(context! {
                album => album,
                images => images,
            }).unwrap()).into_response()
        },
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}