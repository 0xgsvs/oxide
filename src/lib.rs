pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod metrics;
pub mod models;
pub mod ratelimit;
pub mod routes;

use axum::{Router, http::HeaderName, middleware, routing::get};
use routes::{health, tasks};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower_http::{
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

use crate::{auth::auth_routes, models::TaskAssignedEvent, ratelimit::rate_limit_middleware};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub task_notifier: mpsc::Sender<TaskAssignedEvent>,
    pub redis_con: redis::aio::MultiplexedConnection,
    pub rate_limit_enabled: bool,
}

#[derive(utoipa::OpenApi)]
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
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Prometheus `/metrics` handler.
async fn metrics_handler() -> String {
    crate::metrics::render()
}

pub fn create_app(state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &axum::http::Request<axum::body::Body>| {
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            tracing::info_span!(
                "http_request",
                method = %req.method(),
                uri = %req.uri(),
                request_id = %request_id,
            )
        })
        .on_response(
            |response: &axum::http::Response<axum::body::Body>,
             latency: std::time::Duration,
             _span: &tracing::Span| {
                tracing::info!(status = %response.status(), "response sent");
                crate::metrics::record(
                    "http",
                    "/",
                    response.status().as_u16(),
                    latency.as_secs_f64(),
                );
            },
        );

    let request_id_layer =
        SetRequestIdLayer::new(HeaderName::from_static("x-request-id"), MakeRequestUuid);

    let state_for_middleware = state.clone();

    Router::new()
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .route("/health", get(health))
        .merge(auth_routes())
        .route("/tasks", get(tasks::list).post(tasks::create))
        .route(
            "/tasks/{id}",
            get(tasks::get_by_id)
                .patch(tasks::update)
                .delete(tasks::delete),
        )
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            state_for_middleware,
            rate_limit_middleware,
        ))
        .layer(trace_layer)
        .layer(request_id_layer)
}

pub async fn task_notification_worker(mut receiver: mpsc::Receiver<TaskAssignedEvent>) {
    while let Some(event) = receiver.recv().await {
        tracing::info!(
            task_id = event.task_id,
            assignee_id = event.assignee_id,
            "sending task assignment notification"
        );
    }
}
