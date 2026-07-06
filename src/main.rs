use std::net::SocketAddr;

use oxide::{AppState, config, create_app, db, task_notification_worker};
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::load();
    let pool = db::create_pool(&config.database_url).await;

    let (task_notifier, task_receiver) = mpsc::channel(100);
    tokio::spawn(task_notification_worker(task_receiver));

    let state = AppState {
        pool,
        task_notifier,
    };
    let app = create_app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl+C, starting graceful shutdown"),
        _ = terminate => info!("received SIGTERM, starting graceful shutdown"),
    }
}
