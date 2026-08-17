use std::collections::HashMap;
use std::sync::Mutex;

use lazymcp::schemars::JsonSchema;
use lazymcp::serde::{Deserialize, Serialize};
use lazymcp::{Json, LazyMcp, State, tool};

/// Task priority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Low priority
    Low,
    /// Normal priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub completed: bool,
}

#[derive(Default)]
struct TaskStore {
    next_id: Mutex<u32>,
    tasks: Mutex<HashMap<u32, Task>>,
}

/// Create a new task.
/// Longer comments also work.
#[tool]
async fn create_task(
    /// Task title or summary
    title: String,
    /// Priority level
    priority: Priority,
    /// List of tags (e.g. ["backend", "urgent"])
    tags: Vec<String>,
    state: State<TaskStore>,
) -> Json<Task> {
    let mut id_guard = state.next_id.lock().unwrap();
    *id_guard += 1;
    let id = *id_guard;

    let task = Task {
        id,
        title,
        priority,
        tags,
        completed: false,
    };

    state.tasks.lock().unwrap().insert(id, task.clone());

    Json(task)
}

/// Retrieve tasks filtered by priority level.
#[tool]
async fn get_tasks_by_priority(
    /// Priority level to filter by
    priority: Priority,
    state: State<TaskStore>,
) -> Json<Vec<Task>> {
    let storage = state.tasks.lock().unwrap();

    let matching_tasks: Vec<Task> = storage
        .values()
        .filter(|t| t.priority == priority)
        .cloned()
        .collect();

    Json(matching_tasks)
}

/// Mark a task as completed by ID.
#[tool]
async fn complete_task(
    /// Numeric ID of the task
    id: u32,
    state: State<TaskStore>,
) -> Result<String, String> {
    let mut storage = state.tasks.lock().unwrap();

    if let Some(task) = storage.get_mut(&id) {
        task.completed = true;
        Ok(format!(
            "Task #{id} ('{}') completed successfully",
            task.title
        ))
    } else {
        Err(format!("Task with ID #{id} not found"))
    }
}

#[lazymcp::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = TaskStore::default();

    LazyMcp::new("task-tracker", "0.1.0")
        .with_state(store)
        .with_tool(create_task_tool)
        .with_tool(get_tasks_by_priority_tool)
        .with_tool(complete_task_tool)
        .serve_stdio()
        .await?;

    Ok(())
}
