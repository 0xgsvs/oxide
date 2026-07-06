use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Task {
    pub id: i32,
    pub workspace_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee_id: Option<i32>,
    pub created_by: i32,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i32>,
    pub created_by: i32,
}
