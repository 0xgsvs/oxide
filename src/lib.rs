pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod routes;

use std::sync::Arc;

use axum::{Router, routing::get};
use routes::{health, tasks};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor,
};

use crate::{auth::auth_routes, models::TaskAssignedEvent};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub task_notifier: mpsc::Sender<TaskAssignedEvent>,
    pub redis_con: redis::aio::MultiplexedConnection,
}

pub fn create_app(state: AppState) -> Router {
    let governor_conf = GovernorConfigBuilder::default()
        .key_extractor(GlobalKeyExtractor)
        .per_second(10)
        .burst_size(20)
        .finish()
        .expect("Failed to build rate limiter config");

    Router::new()
        .route("/health", get(health))
        .merge(auth_routes())
        .route("/tasks", get(tasks::list).post(tasks::create))
        .route(
            "/tasks/{id}",
            get(tasks::get_by_id)
                .patch(tasks::update)
                .delete(tasks::delete),
        )
        .with_state(state)
        .layer(GovernorLayer::new(Arc::new(governor_conf)))
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
