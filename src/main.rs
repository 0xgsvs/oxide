mod config;
mod db;
mod models;
mod routes;

use std::net::SocketAddr;

use axum::{Router, routing::get};
use routes::{
    health,
    tasks::{create, delete as delete_task, get_by_id, list, update},
};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::load();
    let pool = db::create_pool(&config.database_url).await;

    let app = Router::new()
        .route("/health", get(health))
        .route("/tasks", get(list).post(create))
        .route(
            "/tasks/{id}",
            get(get_by_id).patch(update).delete(delete_task),
        )
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
