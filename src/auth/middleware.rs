use crate::types::{AppState, Claims};
use axum::{extract::State, response::Redirect};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::sync::Arc;
use tower_cookies::Cookies;

/// Decodes the provided JWT token to extract the claims.
///
/// # Arguments
/// * `token` - JWT token to be decoded.
/// * `jwt_secret` - Secret key used for decoding the token.
///
/// # Returns
/// * `bool` - `true` if the token is valid, `false` otherwise.
///
pub async fn is_authenticated(cookies: &Cookies, jwt_secret: &str) -> bool {
    if let Some(token) = cookies.get("auth_token") {
        let claims = decode::<Claims>(
            token.value(),
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &Validation::default(),
        );

        return claims.is_ok();
    }
    false
}

/// Middleware to require authentication for a route.
///
/// # Arguments
/// * `cookies` - Cookies extracted from the request.
/// * `state` - Application state containing the JWT secret key.
///
/// # Returns
/// * `Result<(), Redirect>` - `Ok(())` if authenticated, `Err(Redirect::to("/login"))` otherwise.
///
pub async fn require_auth(cookies: Cookies, state: State<Arc<AppState>>) -> Result<(), Redirect> {
    if is_authenticated(&cookies, &state.jwt_secret).await {
        Ok(())
    } else {
        Err(Redirect::to("/login"))
    }
}
