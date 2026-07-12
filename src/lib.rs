pub mod auth;
pub mod cache;
pub mod config;
pub mod error;
pub mod metrics;
pub mod models;
pub mod ratelimit;
pub mod routes;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    body::Body,
    http::{HeaderName, Request, Response},
    middleware::{self, Next},
    response::{Html, IntoResponse},
    routing::get,
};
use routes::{
    health,
    tasks::{create, delete, get_by_id, list, update},
};
use sqlx::PgPool;
use tokio::sync::mpsc::{Receiver, Sender};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    normalize_path::NormalizePathLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{Span, info, info_span};
use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme::Bearer, HttpBuilder, SecurityScheme},
};

use crate::{auth::auth_routes, models::TaskAssignedEvent, ratelimit::rate_limit_middleware};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<str>, // Perf book: Arc avoids cloning string content when AppState is cloned
    pub task_notifier: Sender<TaskAssignedEvent>,
    pub redis_con: redis::aio::MultiplexedConnection,
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Oxide API", description = "Multi-tenant task tracker", version = "0.1.0"),
    paths(
        crate::routes::tasks::create,
        crate::routes::tasks::list,
        crate::routes::tasks::get_by_id,
        crate::routes::tasks::update,
        crate::routes::tasks::delete,
        crate::auth::register_handler,
        crate::auth::login_handler,
    ),
    components(schemas(
        crate::models::Task,
        crate::models::CreateTaskRequest,
        crate::models::UpdateTaskRequest,
        crate::models::AuthResponse,
        crate::models::LoginRequest,
        crate::models::RegisterRequest,
        crate::models::ErrorResponse,
    )),
    tags(
        (name = "tasks", description = "Task management"),
        (name = "auth", description = "Authentication"),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Prometheus `/metrics` handler.
async fn metrics_handler() -> String {
    metrics::render()
}

/// Serve the OpenAPI spec as JSON.
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Serve a docs UI page that loads from CDN.
async fn docs_page() -> Html<&'static str> {
    Html(include_str!("docs.html"))
}

/// Axum middleware that records Prometheus metrics for every request.
async fn metrics_middleware(req: Request<Body>, next: Next) -> impl IntoResponse {
    let start = Instant::now();
    // ponytail: .to_string() necessary — req is consumed by next.run() below,
    // we can't borrow from it after the await point. Two small allocs per request.
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    metrics::inc_active();
    let response = next.run(req).await;
    metrics::dec_active();
    metrics::record(
        &method,
        &path,
        response.status().as_u16(),
        start.elapsed().as_secs_f64(),
    );
    response
}

pub fn create_app(state: AppState, enable_rate_limit: bool, timeout: Duration) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request<Body>| {
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            info_span!(
                "http_request",
                method = %req.method(),
                uri = %req.uri(),
                request_id = %request_id,
            )
        })
        .on_response(
            |response: &Response<Body>, latency: Duration, _span: &Span| {
                info!(status = %response.status(), latency = ?latency, "response sent");
            },
        );

    let request_id_layer =
        SetRequestIdLayer::new(HeaderName::from_static("x-request-id"), MakeRequestUuid);

    let state_for_rate_limit = state.clone();

    let router = Router::new()
        .route("/docs", get(docs_page))
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/health", get(health))
        .merge(auth_routes())
        .route("/tasks", get(list).post(create))
        .route("/tasks/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .layer(trace_layer)
        .layer(request_id_layer)
        .layer(SetSensitiveRequestHeadersLayer::new([
            HeaderName::from_static("authorization"),
        ]))
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CatchPanicLayer::new())
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn(metrics_middleware))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            timeout,
        ));

    if enable_rate_limit {
        router.layer(middleware::from_fn_with_state(
            state_for_rate_limit,
            rate_limit_middleware,
        ))
    } else {
        router
    }
}

pub async fn task_notification_worker(mut receiver: Receiver<TaskAssignedEvent>) {
    while let Some(event) = receiver.recv().await {
        info!(
            task_id = event.task_id,
            assignee_id = event.assignee_id,
            "sending task assignment notification"
        );
    }
}
