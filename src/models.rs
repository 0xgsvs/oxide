use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct Task {
    pub id: i32,
    pub workspace_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee_id: Option<i32>,
    pub created_by: i32,
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i32>,
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct UpdateTaskRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub assignee_id: Option<i32>,
}

impl UpdateTaskRequest {
    #[must_use]
    pub fn status_is_valid(&self) -> bool {
        match &self.status {
            None => true,
            Some(s) => matches!(s.as_str(), "todo" | "in_progress" | "done"),
        }
    }
}

#[derive(Debug)]
pub struct TaskAssignedEvent {
    pub task_id: i32,
    pub assignee_id: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}
