use axum::{
    Json, body::Body, extract::State, http::StatusCode, middleware::Next, response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde_json::json;

use crate::{AppState, auth::Claims};

/// Check rate limit for a given Redis key using INCR + EXPIRE.
/// Returns `true` if the request is allowed.
async fn check_rate_limit(
    con: &mut redis::aio::MultiplexedConnection,
    key: &str,
    max_requests: u64,
    window_secs: u64,
) -> bool {
    // Atomic: INCR then conditionally EXPIRE on first hit.
    let count: Result<i64, redis::RedisError> = redis::cmd("INCR").arg(key).query_async(con).await;

    match count {
        Ok(1) => {
            // First request in this window — set expiry.
            let _: Result<(), _> = redis::cmd("EXPIRE")
                .arg(key)
                .arg(window_secs as i64)
                .query_async(con)
                .await;
            true
        }
        Ok(n) if n as u64 <= max_requests => true,
        Ok(_) => false,
        Err(e) => {
            tracing::error!(%e, "rate limit Redis error");
            true // allow on error (fail open)
        }
    }
}

/// Extract user ID from Authorization header without the full AuthUser extractor.
fn extract_user_id(request: &axum::http::Request<Body>, jwt_secret: &str) -> Option<i32> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;

    Some(token_data.claims.sub)
}

/// Axum middleware for per-user rate limiting.
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if !state.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    let key = if let Some(uid) = extract_user_id(&request, &state.jwt_secret) {
        format!("ratelimit:user:{uid}")
    } else {
        let ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next().map(str::trim))
            .map(|s| s.to_string())
            .or_else(|| {
                request
                    .extensions()
                    .get::<std::net::SocketAddr>()
                    .map(|addr| addr.ip().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());
        format!("ratelimit:ip:{}", ip)
    };

    let (max_reqs, window) = {
        let path = request.uri().path();
        if path.starts_with("/auth/register") {
            (20u64, 3600u64) // 20 per hour — accommodates org onboarding
        } else if path.starts_with("/auth/login") {
            (30u64, 60u64) // 30 per minute — handles office NAT bursts
        } else {
            (60u64, 60u64) // 60 per minute — per-user for authenticated, per-IP for anonymous
        }
    };

    let mut con = state.redis_con.clone();
    if !check_rate_limit(&mut con, &key, max_reqs, window).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": "Rate limit exceeded. Try again later."})),
        ));
    }

    Ok(next.run(request).await)
}
