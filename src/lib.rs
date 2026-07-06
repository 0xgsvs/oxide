pub mod config;
pub mod db;
pub mod models;
pub mod routes;

use axum::{Router, routing::get};
use routes::{health, tasks};
use sqlx::PgPool;

pub fn create_app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tasks", get(tasks::list).post(tasks::create))
        .route(
            "/tasks/{id}",
            get(tasks::get_by_id)
                .patch(tasks::update)
                .delete(tasks::delete),
        )
        .with_state(pool)
}
