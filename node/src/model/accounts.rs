use serde::Serialize;
use uuid::Uuid;
use strum_macros::{EnumString, Display};

#[derive(Serialize, EnumString, Display, Eq, PartialEq)]
pub enum TaskState {
    NotStarted,
    InProgress,
    Completed,
    Paused,
    Failed
}

#[derive(Serialize)]
pub struct Task {
    pub user_uuid: String,
    pub task_uuid: String,
    pub task_type: String,
    pub state: TaskState,
    pub source_file: String,
    pub result_file: Option<String>
}