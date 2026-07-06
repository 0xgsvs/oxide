use std::net::SocketAddr;

use oxide::{config, create_app, db};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::load();
    let pool = db::create_pool(&config.database_url).await;
    let app = create_app(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on: {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
