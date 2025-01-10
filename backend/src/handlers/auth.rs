use crate::config::CONFIG;
use crate::models::{AppState, Claims, LoginRequest, LoginResponse};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use bcrypt::{verify, BcryptResult};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::{error, info, warn};

// Password verification helper
async fn verify_password(password: &str, hash: &str) -> BcryptResult<bool> {
    let password = password.to_string();
    let hash = hash.to_string();
    match tokio::task::spawn_blocking(move || {
        verify(&password, &hash)
    }).await {
        Ok(result) => result,
        Err(_) => Ok(false),
    }
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    info!("Attempting login with username: {}", payload.username);

    if payload.username != state.admin_credentials.username {
        info!("Invalid username: {}", payload.username);
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Invalid credentials"
        }))).into_response();
    }

    match verify_password(&payload.password, &state.admin_credentials.password_hash).await {
        Ok(is_valid) if is_valid => {
            info!("Password verified for username: {}", payload.username);
            let username = payload.username.clone();
            let claims = Claims {
                sub: username.clone(),
                exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(CONFIG.jwt_secret.as_bytes()),
            ).unwrap();
            info!("Token generated for username: {}", username);
            info!("Token generated for username: {}", payload.username);
            (StatusCode::OK, Json(LoginResponse { token })).into_response()
        },
        _ => {
            info!("Invalid password for username: {}", payload.username);
            (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid credentials"
            }))).into_response()
        }
    }
}

pub async fn verify_handler(headers: HeaderMap) -> impl IntoResponse {
    info!("Verifying token");
    let auth_header = match headers.get("Authorization") {
        Some(header) => header.to_str().unwrap_or(""),
        None => {
            warn!("No authorization header");
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "No authorization header"
            }))).into_response();
        }
    };

    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid authorization header format");
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Invalid authorization header"
        }))).into_response();
    }

    let token = &auth_header["Bearer ".len()..];
    info!("Verifying token: {}", token);

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(CONFIG.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(_) => {
            info!("Token is valid");
            (StatusCode::OK, Json(serde_json::json!({
                "valid": true
            }))).into_response()
        },
        Err(e) => {
            error!("Token verification failed: {:?}", e);
            (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid token"
            }))).into_response()
        },
    }
}

pub fn verify_admin_request(headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    info!("Verifying admin request");
    let auth_header = match headers.get("Authorization") {
        Some(header) => header.to_str().unwrap_or(""),
        None => {
            warn!("No authorization header");
            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "No authorization header"
            }))));
        }
    };

    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid authorization header format");
        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Invalid authorization header"
        }))));
    }

    let token = &auth_header["Bearer ".len()..];
    info!("Verifying admin token: {}", token);

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(CONFIG.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Token verification failed: {:?}", e);
            Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid token"
            }))))
        },
    }
}