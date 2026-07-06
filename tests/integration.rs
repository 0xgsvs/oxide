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

fn app(pool: PgPool) -> (Router, mpsc::Receiver<TaskAssignedEvent>) {
    let (task_notifier, task_receiver) = mpsc::channel(100);
    let state = AppState {
        pool,
        task_notifier,
    };
    (create_app(state), task_receiver)
}

async fn seed_user_and_workspace(pool: &PgPool) {
    sqlx::query!(
        "INSERT INTO users (email, password_hash, role) VALUES ('test@example.com', 'hash', \
         'admin')"
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!("INSERT INTO workspaces (name, owner_id) VALUES ('Default Workspace', 1)")
        .execute(pool)
        .await
        .unwrap();
}

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test]
async fn health_check(pool: PgPool) {
    let (app, _rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn create_task(pool: PgPool) {
    seed_user_and_workspace(&pool).await;

    let (app, _rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tasks")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"workspace_id": 1, "title": "Test task", "created_by": 1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    assert_eq!(body["title"], "Test task");
    assert_eq!(body["status"], "todo");
}

#[sqlx::test]
async fn create_task_sends_assignment_notification(pool: PgPool) {
    seed_user_and_workspace(&pool).await;

    let (app, mut rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tasks")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workspace_id": 1,
                        "title": "Assigned task",
                        "created_by": 1,
                        "assignee_id": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let event = rx.recv().await.expect("expected assignment notification");
    assert_eq!(event.assignee_id, 1);
}

#[sqlx::test]
async fn list_tasks(pool: PgPool) {
    seed_user_and_workspace(&pool).await;

    sqlx::query!("INSERT INTO tasks (workspace_id, title, created_by) VALUES (1, 'Task 1', 1)")
        .execute(&pool)
        .await
        .unwrap();

    let (app, _rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    assert!(body.as_array().unwrap().len() >= 1);
}

#[sqlx::test]
async fn update_task(pool: PgPool) {
    seed_user_and_workspace(&pool).await;

    sqlx::query!("INSERT INTO tasks (workspace_id, title, created_by) VALUES (1, 'Task 1', 1)")
        .execute(&pool)
        .await
        .unwrap();

    let (app, _rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/tasks/1")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"title": "Updated task", "status": "in_progress"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    assert_eq!(body["title"], "Updated task");
    assert_eq!(body["status"], "in_progress");
}

#[sqlx::test]
async fn delete_task(pool: PgPool) {
    seed_user_and_workspace(&pool).await;

    sqlx::query!("INSERT INTO tasks (workspace_id, title, created_by) VALUES (1, 'Task 1', 1)")
        .execute(&pool)
        .await
        .unwrap();

    let (app, _rx) = app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/tasks/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
