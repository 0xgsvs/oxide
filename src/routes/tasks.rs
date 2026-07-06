use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::models::{CreateTaskRequest, Task};

#[derive(Deserialize)]
pub struct ListTasksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn create(State(pool): State<PgPool>, Json(req): Json<CreateTaskRequest>) -> Json<Task> {
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
        req.assignee_id,
        req.created_by
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(task)
}

pub async fn list(
    State(pool): State<PgPool>,
    Query(query): Query<ListTasksQuery>,
) -> Json<Vec<Task>> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

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
    .fetch_all(&pool)
    .await
    .unwrap();

    Json(tasks)
}

pub async fn get_by_id(State(pool): State<PgPool>, Path(id): Path<i32>) -> Json<Task> {
    let task = sqlx::query_as!(
        Task,
        r#"
        SELECT id, workspace_id, title, description, status, assignee_id, created_by
        FROM tasks
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(task)
}
