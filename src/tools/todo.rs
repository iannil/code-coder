/// ─── TodoTool ──────────────────────────────────────────────────────────────
///
/// Task management tool.  Agent can create, list, update, complete, and
/// delete tasks.  State is in-memory (per-session).
///
/// Input format (JSON):
///   {"action": "list"}
///   {"action": "create", "task": "Do X"}
///   {"action": "update", "id": 0, "task": "Do Y"}
///   {"action": "complete", "id": 0}
///   {"action": "delete", "id": 0}

use super::Tool;
use std::sync::atomic::{AtomicU64, Ordering};

static TODO_COUNTER: AtomicU64 = AtomicU64::new(1);

/// In-memory task store (global, per-process).
static TASKS: std::sync::LazyLock<std::sync::Mutex<Vec<TodoItem>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TodoItem {
    id: u64,
    task: String,
    done: bool,
}

pub struct TodoTool;

impl Tool for TodoTool {
    fn name(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Manage tasks. Input JSON: {\"action\":\"list\"|\"create\"|\"update\"|\"complete\"|\"delete\", \"task\":\"...\", \"id\":0}"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct TodoInput {
            action: String,
            #[serde(default)]
            task: String,
            #[serde(default)]
            id: u64,
        }

        let parsed: TodoInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid todo input: {e}"))?;

        let mut tasks = TASKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        match parsed.action.as_str() {
            "list" => {
                if tasks.is_empty() {
                    return Ok("No tasks.".into());
                }
                let mut out = String::from("── Tasks ──\n");
                for (i, t) in tasks.iter().enumerate() {
                    let status = if t.done { "✓" } else { " " };
                    out.push_str(&format!("  {i}. [{status}] #{} {}\n", t.id, t.task));
                }
                Ok(out)
            }
            "create" => {
                if parsed.task.is_empty() {
                    anyhow::bail!("task field required for create");
                }
                let id = TODO_COUNTER.fetch_add(1, Ordering::SeqCst);
                tasks.push(TodoItem {
                    id,
                    task: parsed.task.clone(),
                    done: false,
                });
                Ok(format!("Created task #{id}: {}", parsed.task))
            }
            "update" => {
                let item = tasks.iter_mut()
                    .find(|t| t.id == parsed.id)
                    .ok_or_else(|| anyhow::anyhow!("Task #{} not found", parsed.id))?;
                if !parsed.task.is_empty() {
                    item.task = parsed.task;
                }
                Ok(format!("Updated task #{}", parsed.id))
            }
            "complete" => {
                let item = tasks.iter_mut()
                    .find(|t| t.id == parsed.id)
                    .ok_or_else(|| anyhow::anyhow!("Task #{} not found", parsed.id))?;
                item.done = true;
                Ok(format!("Completed task #{}", parsed.id))
            }
            "delete" => {
                let pos = tasks.iter().position(|t| t.id == parsed.id)
                    .ok_or_else(|| anyhow::anyhow!("Task #{} not found", parsed.id))?;
                tasks.remove(pos);
                Ok(format!("Deleted task #{}", parsed.id))
            }
            _ => anyhow::bail!("Unknown action: {}. Use: list, create, update, complete, delete", parsed.action),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Serialize all todo tests to avoid global state races
    static TEST_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));
    fn acquire_test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn setup() {
        let mut tasks = TASKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        tasks.clear();
        TODO_COUNTER.store(1, Ordering::SeqCst);
        drop(tasks);
    }

    #[test]
    fn test_todo_list_empty() {
        let _lock = acquire_test_lock();
        setup();
        let tool = TodoTool;
        let result = tool.execute(r#"{"action": "list"}"#).unwrap();
        assert!(result.contains("No tasks"));
    }

    #[test]
    fn test_todo_create_and_list() {
        let _lock = acquire_test_lock();
        setup();

        let tool = TodoTool;
        let r1 = tool.execute(r#"{"action": "create", "task": "write tests"}"#).unwrap();
        assert!(r1.contains("Created"));

        let r2 = tool.execute(r#"{"action": "list"}"#).unwrap();
        assert!(r2.contains("write tests"));
    }

    #[test]
    fn test_todo_complete() {
        let _lock = acquire_test_lock();
        setup();

        let tool = TodoTool;
        let create_resp = tool.execute(r#"{"action": "create", "task": "fix bug"}"#).unwrap();
        let created_id: u64 = create_resp.split('#').nth(1)
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1);

        let complete = tool.execute(&format!(r#"{{"action": "complete", "id": {}}}"#, created_id)).unwrap();
        assert!(complete.contains("Completed"));
    }

    #[test]
    fn test_todo_update() {
        let _lock = acquire_test_lock();
        setup();

        let tool = TodoTool;
        let create_resp = tool.execute(r#"{"action": "create", "task": "old task"}"#).unwrap();
        let created_id: u64 = create_resp.split('#').nth(1)
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1);

        let r = tool.execute(&format!(r#"{{"action": "update", "id": {}, "task": "updated task"}}"#, created_id)).unwrap();
        assert!(r.contains("Updated"));
        let list = tool.execute(r#"{"action": "list"}"#).unwrap();
        assert!(list.contains("updated task"));
    }

    #[test]
    fn test_todo_delete() {
        let _lock = acquire_test_lock();
        setup();

        let tool = TodoTool;
        let create_resp = tool.execute(r#"{"action": "create", "task": "delete me"}"#).unwrap();
        let created_id: u64 = create_resp.split('#').nth(1)
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1);

        let r = tool.execute(&format!(r#"{{"action": "delete", "id": {}}}"#, created_id)).unwrap();
        assert!(r.contains("Deleted"));
    }

    #[test]
    fn test_todo_invalid_action() {
        let _lock = acquire_test_lock();
        let tool = TodoTool;
        let result = tool.execute(r#"{"action": "bad"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_todo_not_found() {
        let _lock = acquire_test_lock();
        setup();

        let tool = TodoTool;
        let r = tool.execute(r#"{"action": "complete", "id": 999}"#);
        assert!(r.is_err());
        let r2 = tool.execute(r#"{"action": "delete", "id": 999}"#);
        assert!(r2.is_err());
        let r3 = tool.execute(r#"{"action": "update", "id": 999}"#);
        assert!(r3.is_err());
    }

    #[test]
    fn test_todo_create_empty_task() {
        let _lock = acquire_test_lock();
        let tool = TodoTool;
        let r = tool.execute(r#"{"action": "create", "task": ""}"#);
        assert!(r.is_err());
    }
}
