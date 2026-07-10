use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use oxide::{AppState, create_app, models::TaskAssignedEvent};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower::ServiceExt;

async fn app(pool: PgPool) -> (Router, mpsc::Receiver<TaskAssignedEvent>) {
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("Invalid REDIS_URL");
    let redis_con = client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");

    let (task_notifier, task_receiver) = mpsc::channel(100);
    let state = AppState {
        pool,
        jwt_secret: "test-secret".to_string(),
        task_notifier,
        redis_con,
    };
    (create_app(state, false), task_receiver)
}

/// Helper: send a JSON request by cloning the app.
async fn request_json(
    app: &Router,
    method: &str,
    uri: &str,
    body: serde_json::Value,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        builder = builder.header("Authorization", format!("Bearer {t}"));
    }
    let req = builder.body(Body::from(body.to_string())).unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let body = {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

/// Helper: send an empty request by cloning the app.
async fn request_empty(app: &Router, method: &str, uri: &str, token: Option<&str>) -> StatusCode {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("Authorization", format!("Bearer {t}"));
    }
    let req = builder.body(Body::empty()).unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

/// Register a test user and return the auth token.
async fn register_user(app: &Router) -> String {
    let (_status, body) = request_json(
        app,
        "POST",
        "/auth/register",
        json!({"email": "test@example.com", "password": "password123"}),
        None,
    )
    .await;
    body["token"].as_str().unwrap().to_string()
}

#[sqlx::test]
async fn health_check(pool: PgPool) {
    let (app, _rx) = app(pool).await;
    let (status, body) = request_json(&app, "GET", "/health", json!(null), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn auth_register_and_login(pool: PgPool) {
    let (app, _rx) = app(pool).await;

    let (status, body) = request_json(
        &app,
        "POST",
        "/auth/register",
        json!({"email": "new@example.com", "password": "secret123"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().unwrap().len() > 0);
    assert_eq!(body["email"], "new@example.com");

    let (status, body) = request_json(
        &app,
        "POST",
        "/auth/login",
        json!({"email": "new@example.com", "password": "secret123"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().unwrap().len() > 0);

    let (status, _body) = request_json(
        &app,
        "POST",
        "/auth/login",
        json!({"email": "new@example.com", "password": "wrong"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn tasks_require_auth(pool: PgPool) {
    let (app, _rx) = app(pool).await;

    let status = request_empty(&app, "GET", "/tasks", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _body) =
        request_json(&app, "POST", "/tasks", json!({"title": "No auth"}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn create_and_list_tasks(pool: PgPool) {
    let (app, _rx) = app(pool).await;
    let token = register_user(&app).await;

    let (status, body) = request_json(
        &app,
        "POST",
        "/tasks",
        json!({"title": "Test task"}),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Test task");

    let (status, body) = request_json(&app, "GET", "/tasks", json!(null), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().len() >= 1);
}

#[sqlx::test]
async fn create_task_rejects_whitespace_title(pool: PgPool) {
    let (app, _rx) = app(pool).await;
    let token = register_user(&app).await;

    let (status, _body) = request_json(
        &app,
        "POST",
        "/tasks",
        json!({"title": "   "}),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn create_task_rejects_empty_title(pool: PgPool) {
    let (app, _rx) = app(pool).await;
    let token = register_user(&app).await;

    let (status, body) =
        request_json(&app, "POST", "/tasks", json!({"title": ""}), Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Validation failed");
}

#[sqlx::test]
async fn create_task_sends_assignment_notification(pool: PgPool) {
    let (app, mut rx) = app(pool).await;
    let token = register_user(&app).await;

    let (status, _body) = request_json(
        &app,
        "POST",
        "/tasks",
        json!({"title": "Assigned task", "assignee_id": 1}),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let event = rx.recv().await.expect("expected assignment notification");
    assert_eq!(event.assignee_id, 1);
}

#[sqlx::test]
async fn update_and_delete_task(pool: PgPool) {
    let (app, _rx) = app(pool).await;
    let token = register_user(&app).await;

    let (status, body) = request_json(
        &app,
        "POST",
        "/tasks",
        json!({"title": "Initial task"}),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let task_id = body["id"].as_i64().unwrap();

    let (status, body) = request_json(
        &app,
        "PATCH",
        &format!("/tasks/{task_id}"),
        json!({"title": "Updated task", "status": "in_progress"}),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Updated task");
    assert_eq!(body["status"], "in_progress");

    let (status, body) = request_json(
        &app,
        "DELETE",
        &format!("/tasks/{task_id}"),
        json!(null),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Task deleted");
}
