use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::instrument;
use validator::Validate;

use crate::{
    AppState,
    auth::AuthUser,
    cache,
    error::AppError,
    models::{CreateTaskRequest, ErrorResponse, Task, TaskAssignedEvent, UpdateTaskRequest},
};

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListTasksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Creates a new task.
#[utoipa::path(
    post,
    path = "/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 200, description = "Task created successfully", body = Task),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(
        ("auth" = [])
    ),
    tag = "tasks",
)]
#[instrument(skip(state, req), fields(task.title = %req.title))]
pub async fn create(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    let AuthUser(claims) = auth;
    req.validate().map_err(AppError::Validation)?;

    // ponytail: workspace derived from auth — no client-supplied workspace_id,
    // no separate permission check needed.
    let task = sqlx::query_as!(
        Task,
        r#"
        INSERT INTO tasks (workspace_id, title, description, status, assignee_id, created_by)
        SELECT id, $1, $2, 'todo', $3, $4
        FROM workspaces WHERE owner_id = $4
        RETURNING id, workspace_id, title, description, status, assignee_id, created_by
        "#,
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
#[utoipa::path(
    get,
    path = "/tasks",
    params(ListTasksQuery),
    responses(
        (status = 200, description = "List of tasks matching query", body = [Task]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(
        ("auth" = [])
    ),
    tag = "tasks",
)]
#[instrument(skip(state))]
pub async fn list(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<Task>>, AppError> {
    let AuthUser(claims) = auth;
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let cache_key = format!(
        "{}:{}:{}:{}",
        cache::TASK_LIST_KEY,
        claims.sub,
        limit,
        offset
    );

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
        SELECT t.id, t.workspace_id, t.title, t.description, t.status, t.assignee_id, t.created_by
        FROM tasks t
        JOIN workspaces w ON w.id = t.workspace_id AND w.owner_id = $1
        ORDER BY t.id
        LIMIT $2 OFFSET $3
        "#,
        claims.sub,
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
#[utoipa::path(
    get,
    path = "/tasks/{id}",
    params(("id" = i32, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task found", body = Task),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    security(
        ("auth" = [])
    ),
    tag = "tasks",
)]
#[instrument(skip(state))]
pub async fn get_by_id(
    _auth: AuthUser,
    State(state): State<AppState>,
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
#[utoipa::path(
    patch,
    path = "/tasks/{id}",
    params(("id" = i32, Path, description = "Task ID")),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated", body = Task),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    security(
        ("auth" = [])
    ),
    tag = "tasks",
)]
#[instrument(skip(state, req))]
pub async fn update(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    req.validate().map_err(AppError::Validation)?;

    if !req.status_is_valid() {
        return Err(AppError::BadRequest(
            "Invalid status: must be todo, in_progress, or done",
        ));
    }

    let AuthUser(claims) = auth;
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
        AND workspace_id IN (SELECT id FROM workspaces WHERE owner_id = $6)
        RETURNING id, workspace_id, title, description, status, assignee_id, created_by
        "#,
        id,
        req.title,
        req.description,
        req.status,
        req.assignee_id,
        claims.sub,
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
#[utoipa::path(
    delete,
    path = "/tasks/{id}",
    params(("id" = i32, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task deleted", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    security(
        ("auth" = [])
    ),
    tag = "tasks",
)]
#[instrument(skip(state))]
pub async fn delete(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let AuthUser(claims) = auth;
    let result = sqlx::query!(
        "DELETE FROM tasks WHERE id = $1 AND workspace_id IN (SELECT id FROM workspaces WHERE \
         owner_id = $2)",
        id,
        claims.sub,
    )
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Task {id} not found")));
    }

    let mut con = state.redis_con.clone();
    cache::del(&mut con, &[&cache::task_key(id), cache::TASK_LIST_KEY]).await;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"message": "Task deleted"})),
    ))
}
