use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    AppState,
    auth::AuthUser,
    cache,
    models::{CreateTaskRequest, Task, TaskAssignedEvent, UpdateTaskRequest},
};

#[derive(Deserialize)]
pub struct ListTasksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Creates a new task.
pub async fn create(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Json<Task> {
    req.validate().expect("Validation failed");

    let task = sqlx::query_as!(
        Task,
        r#"
        INSERT INTO tasks (workspace_id, title, description, status, assignee_id, created_by)
        VALUES ($1, $2, $3, 'todo', $4, $5)
        RETURNING id, workspace_id, title, description, status, assignee_id, created_by
        "#,
        req.workspace_id,
        req.title,
        req.description,
        req.assignee_id.or(Some(claims.sub)),
        claims.sub,
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();

    // Invalidate task list cache
    let mut con = state.redis_con.clone();
    cache::del(&mut con, &[cache::TASK_LIST_KEY]).await;

    if let Some(assignee_id) = req.assignee_id {
        let _ = state
            .task_notifier
            .send(TaskAssignedEvent {
                task_id: task.id,
                assignee_id,
            })
            .await;
    }

    Json(task)
}

/// Lists tasks with optional pagination.
pub async fn list(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Json<Vec<Task>> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let cache_key = format!("{}:{}:{}", cache::TASK_LIST_KEY, limit, offset);

    // Try cache
    let mut con = state.redis_con.clone();
    if let Some(Ok(tasks)) = cache::get_string(&mut con, &cache_key)
        .await
        .map(|cached| serde_json::from_str::<Vec<Task>>(&cached))
    {
        return Json(tasks);
    }

    // Cache miss — query DB
    let tasks = sqlx::query_as!(
        Task,
        r#"
        SELECT id, workspace_id, title, description, status, assignee_id, created_by
        FROM tasks
        ORDER BY id
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await
    .unwrap();

    // Store in cache
    if let Ok(json) = serde_json::to_string(&tasks) {
        cache::set_string(
            &mut con,
            &cache_key,
            &json,
            std::time::Duration::from_secs(30),
        )
        .await;
    }

    Json(tasks)
}

/// Gets a single task by ID.
pub async fn get_by_id(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Json<Task> {
    let cache_key = cache::task_key(id);

    // Try cache
    let mut con = state.redis_con.clone();
    if let Some(Ok(task)) = cache::get_string(&mut con, &cache_key)
        .await
        .map(|cached| serde_json::from_str::<Task>(&cached))
    {
        return Json(task);
    }

    // Cache miss — query DB
    let task = sqlx::query_as!(
        Task,
        r#"
        SELECT id, workspace_id, title, description, status, assignee_id, created_by
        FROM tasks
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();

    // Store in cache
    if let Ok(json) = serde_json::to_string(&task) {
        cache::set_string(
            &mut con,
            &cache_key,
            &json,
            std::time::Duration::from_secs(60),
        )
        .await;
    }

    Json(task)
}

/// Updates a task.
pub async fn update(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskRequest>,
) -> Json<Task> {
    req.validate().expect("Validation failed");
    assert!(req.status_is_valid(), "Invalid status");

    let task = sqlx::query_as!(
        Task,
        r#"
        UPDATE tasks
        SET
            title = COALESCE($2, title),
            description = COALESCE($3, description),
            status = COALESCE($4, status),
            assignee_id = $5,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, workspace_id, title, description, status, assignee_id, created_by
        "#,
        id,
        req.title,
        req.description,
        req.status,
        req.assignee_id
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();

    // Invalidate cache
    let mut con = state.redis_con.clone();
    cache::del(&mut con, &[&cache::task_key(id), cache::TASK_LIST_KEY]).await;

    if let Some(assignee_id) = req.assignee_id {
        let _ = state
            .task_notifier
            .send(TaskAssignedEvent {
                task_id: task.id,
                assignee_id,
            })
            .await;
    }

    Json(task)
}

/// Deletes a task by ID.
pub async fn delete(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> StatusCode {
    let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
        .execute(&state.pool)
        .await
        .unwrap();

    if result.rows_affected() == 0 {
        return StatusCode::NOT_FOUND;
    }

    // Invalidate cache
    let mut con = state.redis_con.clone();
    cache::del(&mut con, &[&cache::task_key(id), cache::TASK_LIST_KEY]).await;

    StatusCode::NO_CONTENT
}
