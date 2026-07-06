use serde::{Deserialize, Serialize};
use validator::Validate;

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

#[derive(Deserialize, Validate)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i32>,
    pub created_by: i32,
}

#[derive(Deserialize, Validate)]
pub struct UpdateTaskRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub assignee_id: Option<i32>,
}

impl UpdateTaskRequest {
    /// Returns `true` if `status` is unset or one of the allowed values.
    #[must_use]
    pub fn status_is_valid(&self) -> bool {
        match &self.status {
            None => true,
            Some(s) => matches!(s.as_str(), "todo" | "in_progress" | "done"),
        }
    }
}
