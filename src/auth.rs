use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{header, request::Parts},
    response::{IntoResponse, Response},
    routing::post,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    AppState,
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

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // If the rate-limit middleware already decoded the JWT, reuse it.
        if let Some(claims) = parts.extensions.get::<Claims>() {
            return Ok(AuthUser(claims.clone()));
        }

        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .or_else(|| {
                parts
                    .headers
                    .get("Cookie")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|c| {
                            let c = c.trim();
                            c.strip_prefix("auth_token=")
                        })
                    })
            })
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
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .expect("Failed to hash password")
        .to_string()
}

/// Verify a password against an Argon2 hash. Returns `true` if valid.
#[must_use]
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    let argon2 = Argon2::default();
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
        exp: (now + Duration::hours(24)).timestamp() as usize,
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
#[instrument(skip(state, req))]
pub async fn register_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Response, AppError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest("Email and password are required"));
    }

    let password_hash = hash_password(&req.password);
    let role = match req.role.as_deref() {
        None | Some("member") => "member",
        Some(_) => return Err(AppError::BadRequest("Only 'member' role is permitted at registration")),
    };

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

    let workspace = sqlx::query!(
        r#"
        INSERT INTO workspaces (name, owner_id)
        VALUES ($1, $2)
        RETURNING id
        "#,
        format!("{}'s workspace", user.email),
        user.id,
    )
    .fetch_one(&state.pool)
    .await?;

    let token = create_token(user.id, &user.email, &user.role, &state.jwt_secret);
    const SECURE: &str = if cfg!(debug_assertions) {
        ""
    } else {
        "; Secure"
    };
    let cookie = format!(
        "auth_token={}; HttpOnly; Path=/; SameSite=Lax{}; Max-Age=86400",
        token, SECURE,
    );
    let mut resp = Json(AuthResponse {
        token,
        user_id: user.id,
        email: user.email,
        role: user.role,
        workspace_id: workspace.id,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(resp)
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
#[instrument(skip(state, req))]
pub async fn login_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Response, AppError> {
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

    let workspace = sqlx::query!(
        r#"
        SELECT id FROM workspaces WHERE owner_id = $1
        ORDER BY id LIMIT 1
        "#,
        user.id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("No workspace found for this user".to_string()))?;

    let token = create_token(user.id, &user.email, &user.role, &state.jwt_secret);
    const SECURE: &str = if cfg!(debug_assertions) {
        ""
    } else {
        "; Secure"
    };
    let cookie = format!(
        "auth_token={}; HttpOnly; Path=/; SameSite=Lax{}; Max-Age=86400",
        token, SECURE,
    );
    let mut resp = Json(AuthResponse {
        token,
        user_id: user.id,
        email: user.email,
        role: user.role,
        workspace_id: workspace.id,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(resp)
}

/// Build the auth routes.
pub fn auth_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
}
