use crate::auth::middleware::is_authenticated;
use crate::types::{AppState, Claims};
use argon2::{PasswordHash, PasswordVerifier};
use axum::http::{header, StatusCode};
use axum::{
    extract::State,
    response::{Html, Redirect, Response},
    Form,
};
use argon2::Argon2;
use jsonwebtoken::{encode, EncodingKey, Header};
use minijinja::context;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, sync::Arc};
use tower_cookies::cookie::time::{Duration, OffsetDateTime};
use tower_cookies::cookie::CookieBuilder;
use tower_cookies::{Cookie, Cookies};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Html<String>, Redirect> {
    // If already authenticated, redirect to admin page
    if is_authenticated(&cookies, &state.jwt_secret).await {
        return Err(Redirect::to("/admin"));
    }

    let reloader_guard = state.reloader.lock().await;
    let env = reloader_guard.acquire_env().unwrap();
    let tmpl = env.get_template("login.html").unwrap();
    let rendered = tmpl
        .render(context! {
            error => Option::<String>::None
        })
        .unwrap();
    Ok(Html(rendered))
}

pub async fn login_post_handler(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Response<axum::body::Body>, Html<String>> {
    let admin_username = env::var("ADMIN_USERNAME").expect("ADMIN_USERNAME must be set");
    let admin_password_hash = env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");

    // Helper function to create error response
    async fn create_error_response(state: &AppState, error_msg: &str) -> Html<String> {
        let reloader_guard = state.reloader.lock().await;
        let env = reloader_guard.acquire_env().unwrap();
        let tmpl = env.get_template("login.html").unwrap();
        let rendered = tmpl
            .render(context! {
                error => error_msg
            })
            .unwrap();
        Html(rendered)
    }

    // Parse the stored hash
    let parsed_hash = match PasswordHash::new(&admin_password_hash) {
        Ok(hash) => hash,
        Err(_) => {
            return Err(create_error_response(&state, "Invalid password hash configuration").await);
        }
    };

    // Verify the provided password against the prehashed password
    let password_matches = Argon2::default()
        .verify_password(form.password.as_bytes(), &parsed_hash)
        .is_ok();

    if form.username == admin_username && password_matches {
        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;

        let claims = Claims {
            sub: form.username,
            exp: expiration,
        };

        let token = match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
        ) {
            Ok(token) => token,
            Err(_) => return Err(create_error_response(&state, "Internal server error").await),
        };

        let response = Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/admin")
            .body(axum::body::Body::empty())
            .unwrap();

        let cookie = CookieBuilder::new("auth_token", token)
            .http_only(true) // Prevents JavaScript access
            .secure(true) // Ensures the cookie is sent over HTTPS
            .path("/") // Defines the cookie scope
            .max_age(Duration::seconds(3600)) // Matches JWT expiration
            .expires(OffsetDateTime::now_utc() + Duration::seconds(3600)) // Sets the expiration time
            .build();

        cookies.add(cookie);
        Ok(response)
    } else {
        Err(create_error_response(&state, "Invalid username or password").await)
    }
}

/// Handles logout by removing the auth_token cookie and redirecting to the login page.
pub async fn logout_handler(
    State(_state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Redirect {
    // Remove the auth_token cookie
    cookies.remove(Cookie::build("auth_token").build());
    // Redirect to the login page
    Redirect::to("/login")
}