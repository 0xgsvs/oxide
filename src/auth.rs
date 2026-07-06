use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::post,
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Claims stored in the JWT.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

/// Request body for login.
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response returned after successful authentication.
#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

/// Request body for registration.
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

/// Extractor that validates a JWT from the `Authorization` header.
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| make_401("Missing authorization token"))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| make_401("Invalid or expired token"))?;

        Ok(AuthUser(token_data.claims))
    }
}

fn make_401(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": msg})),
    )
}

fn make_400(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": msg})),
    )
}

/// Hash a password using Argon2.
pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::PasswordHasher;

    let argon2 = argon2::Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .expect("Failed to hash password")
        .to_string()
}

/// Verify a password against an Argon2 hash. Returns `true` if valid.
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordVerifier, phc::PasswordHash};

    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    let argon2 = argon2::Argon2::default();
    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Create a JWT for the given user.
pub fn create_token(user_id: i32, email: &str, role: &str, secret: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to encode JWT")
}

/// POST /auth/register
async fn register_handler(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(make_400("Email and password are required"));
    }

    let password_hash = hash_password(&req.password);
    let role = req.role.as_deref().unwrap_or("member");

    let user = sqlx::query!(
        r#"
        INSERT INTO users (email, password_hash, role)
        VALUES ($1, $2, $3)
        RETURNING id, email, role
        "#,
        req.email,
        password_hash,
        role,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "Email already exists"})),
        )
    })?;

    let token = create_token(user.id, &user.email, &user.role, &state.jwt_secret);
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        email: user.email,
        role: user.role,
    }))
}

/// POST /auth/login
async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query!(
        r#"SELECT id, email, password_hash, role FROM users WHERE email = $1"#,
        req.email,
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| make_400("Database error"))?
    .ok_or_else(|| make_401("Invalid email or password"))?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(make_401("Invalid email or password"));
    }

    let token = create_token(user.id, &user.email, &user.role, &state.jwt_secret);
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        email: user.email,
        role: user.role,
    }))
}

/// Build the auth routes.
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
}
