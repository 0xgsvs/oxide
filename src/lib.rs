pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod metrics;
pub mod models;
pub mod ratelimit;
pub mod routes;

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{HeaderName, Request, Response},
    middleware,
    routing::get,
};
use routes::{
    health,
    tasks::{create, delete, get_by_id, list, update},
};
use sqlx::PgPool;
use tokio::sync::mpsc::{Receiver, Sender};
use tower_http::{
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{Span, info, info_span};
use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme::Bearer, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

use crate::{auth::auth_routes, models::TaskAssignedEvent, ratelimit::rate_limit_middleware};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub task_notifier: Sender<TaskAssignedEvent>,
    pub redis_con: redis::aio::MultiplexedConnection,
    pub rate_limit_enabled: bool,
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

pub fn create_app(state: AppState) -> Router {
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
                info!(status = %response.status(), "response sent");
                metrics::record(
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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health))
        .merge(auth_routes())
        .route("/tasks", get(list).post(create))
        .route("/tasks/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            state_for_middleware,
            rate_limit_middleware,
        ))
        .layer(trace_layer)
        .layer(request_id_layer)
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
