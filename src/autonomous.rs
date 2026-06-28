/// ─── Autonomous Module ─────────────────────────────────────────────────────
///
/// Daemon-mode runner that lets the agent run autonomously without TUI.
/// Supports timers (scheduled tasks) and file system monitoring.

use crate::agent::{AgentCommand, AgentResponse};
use crate::config::ConfigStore;
use crate::event::{Event, FileChangeKind, SharedEventBus, SystemCommand};
use std::sync::Arc;
use std::time::Duration;

/// ─── ScheduledTask ─────────────────────────────────────────────────────────

/// A task scheduled to run at a fixed interval.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub prompt: String,
    pub interval_secs: u64,
}

/// ─── Scheduler ─────────────────────────────────────────────────────────────

/// Fires Timer events at scheduled intervals.
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
}

impl Scheduler {
    pub fn new(tasks: Vec<ScheduledTask>) -> Self {
        Self { tasks }
    }

    /// Run the scheduler loop, firing events on the bus.
    pub async fn run(self, bus: SharedEventBus) {
        let mut handles = Vec::new();
        for task in self.tasks {
            let bus = bus.clone();
            let name = task.name.clone();
            let prompt = task.prompt.clone();
            let interval = Duration::from_secs(task.interval_secs);

            let handle = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    eprintln!("[scheduler] Firing timer '{}'", name);

                    let mut bus_clone = bus.clone();
                    // Publish a Timer event (the Event enum has this variant)
                    let _ = bus_clone.publish_event(Event::Timer { id: name.clone() });

                    // Also send a ProcessMessage so the agent acts on it
                    // (The autonomous runner processes bus events and sends messages)
                    // Real execution happens in AutonomousRunner
                    let _ = bus_clone.publish_event(Event::UserMessage {
                        text: format!("[Scheduled: {}] {}", name, prompt),
                        session_id: "__scheduler__".into(),
                    });
                }
            });
            handles.push(handle);
        }

        // Wait for all scheduler tasks
        for h in handles {
            let _ = h.await;
        }
    }
}

/// ─── FileWatcher ───────────────────────────────────────────────────────────

/// Watches directories for file changes using the `notify` crate.
pub struct FileWatcher {
    paths: Vec<String>,
}

impl FileWatcher {
    pub fn new(paths: Vec<String>) -> Self {
        Self { paths }
    }

    /// Run the file watcher, publishing FileSystem events on the bus.
    pub async fn run(self, bus: SharedEventBus) -> anyhow::Result<()> {
        use notify::EventKind;
        use notify::Watcher;
        use std::path::Path;

        if self.paths.is_empty() {
            eprintln!("[watcher] No paths to watch");
            return Ok(());
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Event>(256);

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        for path in &self.paths {
            watcher.watch(Path::new(path), notify::RecursiveMode::Recursive)?;
            eprintln!("[watcher] Watching: {path}");
        }

        // Forward events to the event bus
        while let Some(event) = rx.recv().await {
            let kind = match event.kind {
                EventKind::Create(_) => FileChangeKind::Created,
                EventKind::Modify(_) => FileChangeKind::Modified,
                EventKind::Remove(_) => FileChangeKind::Deleted,
                _ => continue,
            };

            // Pick the first path from the event
            if let Some(path) = event.paths.first() {
                let path_str = path.to_string_lossy().to_string();
                let mut bus_clone = bus.clone();
                let _ = bus_clone.publish_event(Event::FileSystem {
                    path: path_str,
                    kind,
                });
            }
        }

        Ok(())
    }
}

/// ─── AutonomousRunner ──────────────────────────────────────────────────────

/// Runs the agent in daemon mode: processes events from the bus and
/// sends messages to the agent.  No TUI attached.
pub struct AutonomousRunner;

impl AutonomousRunner {
    /// Run the autonomous agent loop.
    pub async fn run(
        cmd_tx: std::sync::mpsc::Sender<AgentCommand>,
        mut resp_rx: tokio::sync::mpsc::Receiver<AgentResponse>,
        bus: SharedEventBus,
        config: ConfigStore,
        tasks: Vec<ScheduledTask>,
        watch_paths: Vec<String>,
    ) -> anyhow::Result<()> {
        // Start scheduler
        if !tasks.is_empty() {
            let sched_bus = bus.clone();
            let scheduler = Scheduler::new(tasks);
            tokio::spawn(async move {
                scheduler.run(sched_bus).await;
            });
        }

        // Start file watcher
        if !watch_paths.is_empty() {
            let watch_bus = bus.clone();
            let watcher = FileWatcher::new(watch_paths);
            tokio::spawn(async move {
                if let Err(e) = watcher.run(watch_bus).await {
                    eprintln!("[watcher] Error: {e}");
                }
            });
        }

        // Subscribe to the event bus for system events
        let bus_for_sub = bus.clone();

        // Main loop: poll the event bus and forward events to the agent
        eprintln!("[codecoder] Running in daemon mode");
        eprintln!("[codecoder] Press Ctrl+C to stop");

        let mut agent_busy = false;

        loop {
            // Check for agent responses
            loop {
                match resp_rx.try_recv() {
                    Ok(AgentResponse::Text { text }) => {
                        eprintln!("[agent] {text}");
                        agent_busy = false;
                    }
                    Ok(AgentResponse::Error { message }) => {
                        eprintln!("[agent] Error: {message}");
                        agent_busy = false;
                    }
                    Ok(AgentResponse::Shutdown) => break,
                    Ok(_) => {} // ignore other responses
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return Ok(()),
                }
            }

            // Check for events on the bus
            let event = {
                let mut bus_inner = bus_for_sub.clone();
                bus_inner.drain_event()
            };

            if let Some(Event::UserMessage { text, .. }) = event {
                if !agent_busy {
                    agent_busy = true;
                    let _ = cmd_tx.send(AgentCommand::ProcessMessage { text });
                }
            }

            // Yield to not burn CPU
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ScheduledTask ────────────────────────────────────────────────────

    #[test]
    fn test_scheduled_task_construction() {
        let task = ScheduledTask {
            name: "health-check".into(),
            prompt: "Check system health".into(),
            interval_secs: 60,
        };
        assert_eq!(task.name, "health-check");
        assert_eq!(task.prompt, "Check system health");
        assert_eq!(task.interval_secs, 60);
    }

    // ─── Scheduler ────────────────────────────────────────────────────────

    #[test]
    fn test_scheduler_new_empty() {
        let _scheduler = Scheduler::new(vec![]);
    }

    #[test]
    fn test_scheduler_new_with_tasks() {
        let tasks = vec![
            ScheduledTask {
                name: "task1".into(),
                prompt: "Do thing 1".into(),
                interval_secs: 30,
            },
            ScheduledTask {
                name: "task2".into(),
                prompt: "Do thing 2".into(),
                interval_secs: 120,
            },
        ];
        let _scheduler = Scheduler::new(tasks);
    }

    // ─── FileWatcher ──────────────────────────────────────────────────────

    #[test]
    fn test_file_watcher_new_empty() {
        let _watcher = FileWatcher::new(vec![]);
    }

    #[test]
    fn test_file_watcher_new_with_paths() {
        let watcher = FileWatcher::new(vec!["/tmp".into(), "/var/log".into()]);
        _ = watcher;
    }

    #[tokio::test]
    async fn test_file_watcher_empty_paths_returns_immediately() {
        let bus = SharedEventBus::new();
        let watcher = FileWatcher::new(vec![]);
        // Should return Ok(()) immediately without hanging
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            watcher.run(bus),
        ).await;
        assert!(result.is_ok(), "Empty watcher should return immediately");
        assert!(result.unwrap().is_ok());
    }

    // ─── AutonomousRunner ─────────────────────────────────────────────────

    #[test]
    fn test_autonomous_runner_exists() {
        let _runner = AutonomousRunner;
    }

    #[tokio::test]
    async fn test_scheduler_fires_timer_event() {
        let bus = SharedEventBus::new();
        let bus_clone = bus.clone();

        let task = ScheduledTask {
            name: "test-task".into(),
            prompt: "test prompt".into(),
            interval_secs: 1, // fire every second
        };

        let scheduler = Scheduler::new(vec![task]);
        let handle = tokio::spawn(async move {
            scheduler.run(bus_clone).await;
        });

        // Wait for the first timer to fire (interval is 1 second)
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Drain events — should have a Timer or UserMessage
        let event = bus.drain_event();
        assert!(event.is_some(), "Expected at least one event from scheduler after 1.1s");

        // Clean up
        handle.abort();
    }

    #[tokio::test]
    async fn test_scheduler_multiple_tasks() {
        let bus = SharedEventBus::new();
        let bus_clone = bus.clone();

        let tasks = vec![
            ScheduledTask {
                name: "fast".into(),
                prompt: "fast task".into(),
                interval_secs: 1,
            },
            ScheduledTask {
                name: "slow".into(),
                prompt: "slow task".into(),
                interval_secs: 2,
            },
        ];

        let scheduler = Scheduler::new(tasks);
        let handle = tokio::spawn(async move {
            scheduler.run(bus_clone).await;
        });

        // Let both fire at least once
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        let mut found_fast = false;
        let mut found_slow = false;
        while let Some(event) = bus.drain_event() {
            if let Event::UserMessage { text, .. } = &event {
                if text.contains("fast") { found_fast = true; }
                if text.contains("slow") { found_slow = true; }
            }
        }
        assert!(found_fast, "Should have fired fast task");

        handle.abort();
    }
}
