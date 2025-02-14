use crate::{db::get_albums_with_oldest_image, types::AppState};
use axum::{extract::State, response::Html};
use minijinja::context;
use std::sync::Arc;

pub async fn home_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let albums = get_albums_with_oldest_image(&state.pool)
        .await
        .unwrap_or_default();

    let reloader_guard = state.reloader.lock().await;
    let env = reloader_guard.acquire_env().unwrap();
    let tmpl = env.get_template("home.html").unwrap();
    let rendered = tmpl.render(context! {
        albums => albums,
    })
    .unwrap();
    Html(rendered)
}
