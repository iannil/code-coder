//! Task queue implementation for autonomous mode
//!
//! Provides priority-based task scheduling with dependency resolution.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Unique task identifier
pub type TaskId = String;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl TaskPriority {
    /// Get numeric value for comparison (higher = more important)
    pub fn value(&self) -> u8 {
        match self {
            TaskPriority::Critical => 4,
            TaskPriority::High => 3,
            TaskPriority::Medium => 2,
            TaskPriority::Low => 1,
        }
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Medium
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Blocked,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub session_id: String,
    pub subject: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub dependencies: Vec<TaskId>,
    pub dependents: Vec<TaskId>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub metadata: HashMap<String, serde_json::Value>,
    pub agent: Option<String>,
}

impl Task {
    /// Create a new task
    pub fn new(
        id: TaskId,
        session_id: String,
        subject: String,
        description: String,
        priority: TaskPriority,
    ) -> Self {
        Self {
            id,
            session_id,
            subject,
            description,
            status: TaskStatus::Pending,
            priority,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None,
            completed_at: None,
            error: None,
            retry_count: 0,
            max_retries: 2,
            metadata: HashMap::new(),
            agent: None,
        }
    }

    /// Check if task can run (dependencies satisfied)
    pub fn can_run(&self, completed: &HashSet<TaskId>) -> bool {
        self.status == TaskStatus::Pending
            && self.dependencies.iter().all(|dep| completed.contains(dep))
    }
}

/// Wrapper for priority queue ordering
#[derive(Debug, Clone)]
struct PriorityTask {
    task_id: TaskId,
    priority: TaskPriority,
    created_at: i64,
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier creation time
        match self.priority.value().cmp(&other.priority.value()) {
            Ordering::Equal => other.created_at.cmp(&self.created_at), // Earlier first
            ord => ord,
        }
    }
}

/// Task queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueConfig {
    pub max_concurrent: usize,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            max_retries: 2,
            retry_delay_ms: 1000,
        }
    }
}

/// Task queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskQueueStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub blocked: usize,
}

/// Task queue for managing autonomous mode tasks
///
/// Handles task dependencies, priorities, and execution order using a priority heap.
pub struct TaskQueue {
    session_id: String,
    tasks: HashMap<TaskId, Task>,
    priority_queue: BinaryHeap<PriorityTask>,
    running: HashSet<TaskId>,
    completed: HashSet<TaskId>,
    config: TaskQueueConfig,
    next_id: u64,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new(session_id: String, config: TaskQueueConfig) -> Self {
        Self {
            session_id,
            tasks: HashMap::new(),
            priority_queue: BinaryHeap::new(),
            running: HashSet::new(),
            completed: HashSet::new(),
            config,
            next_id: 1,
        }
    }

    /// Create with default config
    pub fn with_defaults(session_id: String) -> Self {
        Self::new(session_id, TaskQueueConfig::default())
    }

    /// Generate a unique task ID
    fn generate_id(&mut self) -> TaskId {
        let id = format!(
            "task_{}_{:07x}",
            chrono::Utc::now().timestamp_millis(),
            self.next_id
        );
        self.next_id += 1;
        id
    }

    /// Add a task to the queue
    pub fn add(&mut self, mut task: Task) -> TaskId {
        // Generate ID if empty
        if task.id.is_empty() {
            task.id = self.generate_id();
        }

        let id = task.id.clone();
        task.session_id = self.session_id.clone();
        task.max_retries = self.config.max_retries;

        // Register reverse dependencies
        for dep_id in &task.dependencies {
            if let Some(dep) = self.tasks.get_mut(dep_id) {
                dep.dependents.push(id.clone());
            }
        }

        // Add to priority queue
        self.priority_queue.push(PriorityTask {
            task_id: id.clone(),
            priority: task.priority,
            created_at: task.created_at,
        });

        self.tasks.insert(id.clone(), task);
        id
    }

    /// Add a task with builder pattern
    pub fn add_task(
        &mut self,
        subject: impl Into<String>,
        description: impl Into<String>,
        priority: TaskPriority,
    ) -> TaskId {
        let task = Task::new(
            String::new(),
            self.session_id.clone(),
            subject.into(),
            description.into(),
            priority,
        );
        self.add(task)
    }

    /// Add task with dependencies
    pub fn add_with_deps(
        &mut self,
        subject: impl Into<String>,
        description: impl Into<String>,
        priority: TaskPriority,
        dependencies: Vec<TaskId>,
    ) -> TaskId {
        let mut task = Task::new(
            String::new(),
            self.session_id.clone(),
            subject.into(),
            description.into(),
            priority,
        );
        task.dependencies = dependencies;
        self.add(task)
    }

    /// Get a task by ID
    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable task by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Get all tasks
    pub fn all(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    /// Get tasks by status
    pub fn by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.status == status)
            .collect()
    }

    /// Get runnable tasks (pending with satisfied dependencies)
    pub fn runnable(&self) -> Vec<&Task> {
        let available_slots = self.config.max_concurrent.saturating_sub(self.running.len());
        if available_slots == 0 {
            return Vec::new();
        }

        let mut runnable: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| t.can_run(&self.completed))
            .collect();

        // Sort by priority (descending) then by creation time (ascending)
        runnable.sort_by(|a, b| {
            match b.priority.value().cmp(&a.priority.value()) {
                Ordering::Equal => a.created_at.cmp(&b.created_at),
                ord => ord,
            }
        });

        runnable.truncate(available_slots);
        runnable
    }

    /// Start a task
    pub fn start(&mut self, id: &str) -> Result<(), &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Pending {
            return Err("Task not pending");
        }

        if !task.can_run(&self.completed) {
            return Err("Dependencies not satisfied");
        }

        task.status = TaskStatus::Running;
        task.started_at = Some(chrono::Utc::now().timestamp_millis());
        self.running.insert(id.to_string());

        Ok(())
    }

    /// Complete a task successfully
    pub fn complete(&mut self, id: &str) -> Result<i64, &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Running {
            return Err("Task not running");
        }

        let now = chrono::Utc::now().timestamp_millis();
        task.status = TaskStatus::Completed;
        task.completed_at = Some(now);

        let duration = now - task.started_at.unwrap_or(now);

        self.running.remove(id);
        self.completed.insert(id.to_string());

        Ok(duration)
    }

    /// Fail a task
    pub fn fail(&mut self, id: &str, error: String, retryable: bool) -> Result<bool, &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        task.error = Some(error);
        task.retry_count += 1;

        self.running.remove(id);

        // Check if we should retry
        if retryable && task.retry_count < task.max_retries {
            task.status = TaskStatus::Pending;
            // Re-add to priority queue for retry
            self.priority_queue.push(PriorityTask {
                task_id: id.to_string(),
                priority: task.priority,
                created_at: task.created_at,
            });
            return Ok(true); // Will retry
        }

        task.status = TaskStatus::Failed;
        task.completed_at = Some(chrono::Utc::now().timestamp_millis());

        Ok(false) // Final failure
    }

    /// Skip a task
    pub fn skip(&mut self, id: &str, reason: Option<String>) -> Result<(), &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Pending && task.status != TaskStatus::Running {
            return Err("Task cannot be skipped");
        }

        task.status = TaskStatus::Skipped;
        task.completed_at = Some(chrono::Utc::now().timestamp_millis());
        task.error = reason;

        self.running.remove(id);

        Ok(())
    }

    /// Block a task
    pub fn block(&mut self, id: &str, reason: Option<String>) -> Result<(), &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Pending && task.status != TaskStatus::Running {
            return Err("Task cannot be blocked");
        }

        task.status = TaskStatus::Blocked;
        task.error = reason;

        self.running.remove(id);

        Ok(())
    }

    /// Unblock a task
    pub fn unblock(&mut self, id: &str) -> Result<(), &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Blocked {
            return Err("Task not blocked");
        }

        task.status = TaskStatus::Pending;
        task.error = None;

        // Re-add to priority queue
        self.priority_queue.push(PriorityTask {
            task_id: id.to_string(),
            priority: task.priority,
            created_at: task.created_at,
        });

        Ok(())
    }

    /// Retry a failed task
    pub fn retry(&mut self, id: &str) -> Result<(), &'static str> {
        let task = self.tasks.get_mut(id).ok_or("Task not found")?;

        if task.status != TaskStatus::Failed {
            return Err("Task not failed");
        }

        task.status = TaskStatus::Pending;
        task.error = None;

        // Re-add to priority queue
        self.priority_queue.push(PriorityTask {
            task_id: id.to_string(),
            priority: task.priority,
            created_at: task.created_at,
        });

        Ok(())
    }

    /// Get queue statistics
    pub fn stats(&self) -> TaskQueueStats {
        let mut stats = TaskQueueStats::default();

        for task in self.tasks.values() {
            stats.total += 1;
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Running => stats.running += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed => stats.failed += 1,
                TaskStatus::Skipped => stats.skipped += 1,
                TaskStatus::Blocked => stats.blocked += 1,
            }
        }

        stats
    }

    /// Check if all tasks are complete
    pub fn is_complete(&self) -> bool {
        let stats = self.stats();
        stats.total > 0
            && stats.pending == 0
            && stats.running == 0
            && stats.blocked == 0
    }

    /// Check if queue has failures
    pub fn has_failures(&self) -> bool {
        self.stats().failed > 0
    }

    /// Get failed tasks
    pub fn failed(&self) -> Vec<&Task> {
        self.by_status(TaskStatus::Failed)
    }

    /// Get task chain (all dependencies and dependents)
    pub fn chain(&self, id: &str) -> Vec<&Task> {
        let mut visited = HashSet::new();

        fn collect_deps(
            tasks: &HashMap<TaskId, Task>,
            id: &str,
            visited: &mut HashSet<TaskId>,
        ) {
            if visited.contains(id) {
                return;
            }
            visited.insert(id.to_string());

            if let Some(task) = tasks.get(id) {
                for dep_id in &task.dependencies {
                    collect_deps(tasks, dep_id, visited);
                }
                for dep_id in &task.dependents {
                    collect_deps(tasks, dep_id, visited);
                }
            }
        }

        collect_deps(&self.tasks, id, &mut visited);

        visited
            .iter()
            .filter_map(|tid| self.tasks.get(tid))
            .collect()
    }

    /// Clear all tasks
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.priority_queue.clear();
        self.running.clear();
        self.completed.clear();
    }

    /// Get config
    pub fn config(&self) -> &TaskQueueConfig {
        &self.config
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Serialize queue state
    pub fn serialize(&self) -> serde_json::Value {
        serde_json::json!({
            "session_id": self.session_id,
            "tasks": self.tasks.values().collect::<Vec<_>>(),
            "config": self.config,
        })
    }

    /// Deserialize queue state
    pub fn deserialize(data: serde_json::Value) -> Result<Self, serde_json::Error> {
        let session_id = data["session_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let config: TaskQueueConfig =
            serde_json::from_value(data["config"].clone()).unwrap_or_default();

        let tasks: Vec<Task> = serde_json::from_value(data["tasks"].clone())?;

        let mut queue = Self::new(session_id, config);

        // Rebuild state
        for mut task in tasks {
            let id = task.id.clone();

            // Reset running tasks to pending
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Pending;
            }

            // Track completed tasks
            if task.status == TaskStatus::Completed {
                queue.completed.insert(id.clone());
            }

            queue.tasks.insert(id.clone(), task.clone());

            // Re-add pending tasks to priority queue
            if task.status == TaskStatus::Pending {
                queue.priority_queue.push(PriorityTask {
                    task_id: id,
                    priority: task.priority,
                    created_at: task.created_at,
                });
            }
        }

        Ok(queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_task() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let id = queue.add_task("Test task", "Description", TaskPriority::Medium);

        let task = queue.get(&id).unwrap();
        assert_eq!(task.subject, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let id = queue.add_task("Task 1", "Desc", TaskPriority::High);

        // Start
        queue.start(&id).unwrap();
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Running);

        // Complete
        let duration = queue.complete(&id).unwrap();
        assert!(duration >= 0);
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_failure_retry() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let id = queue.add_task("Failing task", "Will fail", TaskPriority::Medium);

        queue.start(&id).unwrap();

        // First failure - should retry
        let will_retry = queue.fail(&id, "Error 1".into(), true).unwrap();
        assert!(will_retry);
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Pending);

        // Second try
        queue.start(&id).unwrap();
        let will_retry = queue.fail(&id, "Error 2".into(), true).unwrap();
        assert!(!will_retry); // max_retries = 2, so second failure is final
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Failed);
    }

    #[test]
    fn test_dependencies() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let task1 = queue.add_task("Task 1", "First", TaskPriority::Medium);
        let task2 = queue.add_with_deps("Task 2", "Depends on 1", TaskPriority::Medium, vec![task1.clone()]);

        // Only task1 should be runnable
        let runnable = queue.runnable();
        assert_eq!(runnable.len(), 1);
        assert_eq!(runnable[0].id, task1);

        // Complete task1
        queue.start(&task1).unwrap();
        queue.complete(&task1).unwrap();

        // Now task2 should be runnable
        let runnable = queue.runnable();
        assert_eq!(runnable.len(), 1);
        assert_eq!(runnable[0].id, task2);
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let _low = queue.add_task("Low", "Low priority", TaskPriority::Low);
        let high = queue.add_task("High", "High priority", TaskPriority::High);
        let critical = queue.add_task("Critical", "Critical priority", TaskPriority::Critical);
        let medium = queue.add_task("Medium", "Medium priority", TaskPriority::Medium);

        let runnable = queue.runnable();
        assert_eq!(runnable.len(), 3); // max_concurrent = 3

        // Should be ordered: critical, high, medium (low excluded due to limit)
        assert_eq!(runnable[0].id, critical);
        assert_eq!(runnable[1].id, high);
        assert_eq!(runnable[2].id, medium);
    }

    #[test]
    fn test_concurrency_limit() {
        let mut queue = TaskQueue::new(
            "test-session".into(),
            TaskQueueConfig {
                max_concurrent: 2,
                ..Default::default()
            },
        );

        let t1 = queue.add_task("Task 1", "Desc", TaskPriority::Medium);
        let t2 = queue.add_task("Task 2", "Desc", TaskPriority::Medium);
        let _t3 = queue.add_task("Task 3", "Desc", TaskPriority::Medium);

        // Start 2 tasks
        queue.start(&t1).unwrap();
        queue.start(&t2).unwrap();

        // No more runnable due to limit
        let runnable = queue.runnable();
        assert!(runnable.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let t1 = queue.add_task("Task 1", "Desc", TaskPriority::Medium);
        let t2 = queue.add_task("Task 2", "Desc", TaskPriority::Medium);
        queue.add_task("Task 3", "Desc", TaskPriority::Medium);

        queue.start(&t1).unwrap();
        queue.complete(&t1).unwrap();

        queue.start(&t2).unwrap();
        queue.fail(&t2, "Error".into(), false).unwrap();

        let stats = queue.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.pending, 1);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let t1 = queue.add_task("Task 1", "Desc", TaskPriority::High);
        queue.add_with_deps("Task 2", "Depends", TaskPriority::Medium, vec![t1.clone()]);

        queue.start(&t1).unwrap();
        queue.complete(&t1).unwrap();

        // Serialize
        let data = queue.serialize();

        // Deserialize
        let restored = TaskQueue::deserialize(data).unwrap();

        assert_eq!(restored.session_id(), "test-session");
        assert_eq!(restored.stats().total, 2);
        assert_eq!(restored.stats().completed, 1);
        assert_eq!(restored.stats().pending, 1);
    }

    #[test]
    fn test_block_unblock() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let id = queue.add_task("Task", "Desc", TaskPriority::Medium);

        queue.block(&id, Some("Waiting for input".into())).unwrap();
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Blocked);

        queue.unblock(&id).unwrap();
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Pending);
    }

    #[test]
    fn test_skip() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let id = queue.add_task("Task", "Desc", TaskPriority::Medium);

        queue.skip(&id, Some("Not needed".into())).unwrap();
        assert_eq!(queue.get(&id).unwrap().status, TaskStatus::Skipped);
        assert_eq!(queue.stats().skipped, 1);
    }

    #[test]
    fn test_is_complete() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        assert!(!queue.is_complete()); // Empty queue is not complete

        let id = queue.add_task("Task", "Desc", TaskPriority::Medium);

        assert!(!queue.is_complete()); // Has pending

        queue.start(&id).unwrap();
        queue.complete(&id).unwrap();

        assert!(queue.is_complete()); // All done
    }

    #[test]
    fn test_chain() {
        let mut queue = TaskQueue::with_defaults("test-session".into());

        let t1 = queue.add_task("Task 1", "First", TaskPriority::Medium);
        let t2 = queue.add_with_deps("Task 2", "Second", TaskPriority::Medium, vec![t1.clone()]);
        let _t3 = queue.add_with_deps("Task 3", "Third", TaskPriority::Medium, vec![t2.clone()]);

        // Unrelated task
        queue.add_task("Task 4", "Unrelated", TaskPriority::Low);

        let chain = queue.chain(&t2);
        assert_eq!(chain.len(), 3); // t1, t2, t3 are in the chain
    }
}
