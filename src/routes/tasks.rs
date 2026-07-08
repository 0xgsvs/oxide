use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::instrument;
use validator::Validate;

use crate::{
    auth::AuthUser,
    cache,
    error::AppError,
    models::{CreateTaskRequest, Task, TaskAssignedEvent, UpdateTaskRequest},
};

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Creates a new task.
#[instrument(skip(state, req), fields(task.title = %req.title))]
pub async fn create(
    AuthUser(claims): AuthUser,
    State(state): State<crate::AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    req.validate().map_err(AppError::Validation)?;

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
    .await?;

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

    Ok(Json(task))
}

/// Lists tasks with optional pagination.
#[instrument(skip(state))]
pub async fn list(
    AuthUser(_): AuthUser,
    State(state): State<crate::AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<Task>>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let cache_key = format!("{}:{}:{}", cache::TASK_LIST_KEY, limit, offset);

    let mut con = state.redis_con.clone();
    if let Some(Ok(tasks)) = cache::get_string(&mut con, &cache_key)
        .await
        .map(|cached| serde_json::from_str::<Vec<Task>>(&cached))
    {
        return Ok(Json(tasks));
    }

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
    .await?;

    if let Ok(json) = serde_json::to_string(&tasks) {
        cache::set_string(
            &mut con,
            &cache_key,
            &json,
            std::time::Duration::from_secs(30),
        )
        .await;
    }

    Ok(Json(tasks))
}

/// Gets a single task by ID.
#[instrument(skip(state))]
pub async fn get_by_id(
    AuthUser(_): AuthUser,
    State(state): State<crate::AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Task>, AppError> {
    let cache_key = cache::task_key(id);

    let mut con = state.redis_con.clone();
    if let Some(Ok(task)) = cache::get_string(&mut con, &cache_key)
        .await
        .map(|cached| serde_json::from_str::<Task>(&cached))
    {
        return Ok(Json(task));
    }

    let task = sqlx::query_as!(
        Task,
        r#"SELECT id, workspace_id, title, description, status, assignee_id, created_by
        FROM tasks WHERE id = $1"#,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))?;

    if let Ok(json) = serde_json::to_string(&task) {
        cache::set_string(
            &mut con,
            &cache_key,
            &json,
            std::time::Duration::from_secs(60),
        )
        .await;
    }

    Ok(Json(task))
}

/// Updates a task.
#[instrument(skip(state, req))]
pub async fn update(
    AuthUser(_): AuthUser,
    State(state): State<crate::AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    req.validate().map_err(AppError::Validation)?;

    if !req.status_is_valid() {
        return Err(AppError::BadRequest(
            "Invalid status: must be todo, in_progress, or done",
        ));
    }

    let task = sqlx::query_as!(
        Task,
        r#"
        UPDATE tasks SET
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
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))?;

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

    Ok(Json(task))
}

/// Deletes a task by ID.
#[instrument(skip(state))]
pub async fn delete(
    AuthUser(_): AuthUser,
    State(state): State<crate::AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Task {id} not found")));
    }

    let mut con = state.redis_con.clone();
    cache::del(&mut con, &[&cache::task_key(id), cache::TASK_LIST_KEY]).await;

    Ok(StatusCode::NO_CONTENT)
}
