use std::net::SocketAddr;

use oxide::{AppState, config, create_app, db, task_notification_worker};
use redis::Client;
use tokio::{net::TcpListener, signal::ctrl_c, spawn, sync::mpsc};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::load();
    let pool = db::create_pool(&config.database_url).await;

    let redis_client = Client::open(config.redis_url).expect("Invalid REDIS_URL");
    let redis_con = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");

    let (task_notifier, task_receiver) = mpsc::channel(100);
    spawn(task_notification_worker(task_receiver));

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        task_notifier,
        redis_con,
    };
    let app = create_app(state, true);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on: {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix;

        unix::signal(unix::SignalKind::terminate())
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
