use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::request::Parts,
    routing::post,
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    error::AppError,
    models::{AuthResponse, ErrorResponse, LoginRequest, RegisterRequest},
};

/// Claims stored in the JWT.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

/// Extractor that validates a JWT from the `Authorization` header.
#[derive(Debug)]
pub struct AuthUser(pub Claims);

impl FromRequestParts<crate::AppState> for AuthUser {
    type Rejection = AppError;

    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("Missing authorization token"))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("Invalid or expired token"))?;

        Ok(AuthUser(token_data.claims))
    }
}

/// Hash a password using Argon2.
#[must_use]
pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::PasswordHasher;

    let argon2 = argon2::Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .expect("Failed to hash password")
        .to_string()
}

/// Verify a password against an Argon2 hash. Returns `true` if valid.
#[must_use]
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordVerifier, phc::PasswordHash};

    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
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
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered", body = AuthResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state))]
pub async fn register_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest("Email and password are required"));
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
    .map_err(|_| AppError::Conflict("Email already exists"))?;

    let token = create_token(user.id, &user.email, &user.role, &state.jwt_secret);
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        email: user.email,
        role: user.role,
    }))
}

/// POST /auth/login
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state))]
pub async fn login_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query!(
        r#"SELECT id, email, password_hash, role FROM users WHERE email = $1"#,
        req.email,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid email or password"))?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(AppError::Unauthorized("Invalid email or password"));
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
pub fn auth_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
}
