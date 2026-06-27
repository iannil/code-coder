use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

/// ─── Event Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Event {
    UserMessage { text: String, session_id: String },
    Timer { id: String },
    FileSystem { path: String, kind: FileChangeKind },
    ToolResult { tool_name: String, output: String, success: bool },
    LlmResponse { text: String, session_id: String },
    System(SystemCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SystemCommand {
    Shutdown,
    ReloadSkills,
    ReloadConfig,
}

/// ─── Subscriber trait ──────────────────────────────────────────────────────

pub trait Subscriber: Send + 'static {
    fn name(&self) -> &str;
    fn handle(&mut self, event: &Event) -> anyhow::Result<()>;
}

/// ─── EventBus trait ────────────────────────────────────────────────────────

pub trait EventBus: Send + 'static {
    fn publish(&mut self, event: Event) -> anyhow::Result<()>;
    fn subscribe(&mut self, sub: Box<dyn Subscriber>);
    fn drain(&mut self) -> anyhow::Result<()>;
}

/// ─── SharedEventBus ────────────────────────────────────────────────────────
///
/// Thread-safe event bus backed by `Arc<Mutex<>>`.
/// Recovers from Mutex poisoning by ignoring the poison flag.
#[derive(Clone)]
pub struct SharedEventBus {
    inner: Arc<Mutex<SharedBusInner>>,
}

struct SharedBusInner {
    subscribers: Vec<Box<dyn Subscriber>>,
    queue: VecDeque<Event>,
}

impl SharedEventBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedBusInner {
                subscribers: Vec::new(),
                queue: VecDeque::new(),
            })),
        }
    }

    /// Lock the mutex, recovering from poison if needed.
    fn lock(&self) -> std::sync::MutexGuard<'_, SharedBusInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Drain all queued events through subscribers.  Returns the count
    /// of events processed.
    pub fn drain(&self) -> usize {
        let mut inner = self.lock();
        let count = inner.queue.len();
        while let Some(event) = inner.queue.pop_front() {
            for sub in &mut inner.subscribers {
                if let Err(e) = sub.handle(&event) {
                    eprintln!("[bus] subscriber '{}' error: {e}", sub.name());
                }
            }
        }
        count
    }

    pub fn pending(&self) -> usize {
        self.lock().queue.len()
    }

    /// Drain a single event from the queue (used by autonomous runner).
    pub fn drain_event(&self) -> Option<Event> {
        let mut inner = self.lock();
        inner.queue.pop_front()
    }

    /// Publish an event to the bus (convenience method).
    pub fn publish_event(&mut self, event: Event) {
        self.lock().queue.push_back(event);
    }
}

impl EventBus for SharedEventBus {
    fn publish(&mut self, event: Event) -> anyhow::Result<()> {
        self.lock().queue.push_back(event);
        Ok(())
    }

    fn subscribe(&mut self, sub: Box<dyn Subscriber>) {
        self.lock().subscribers.push(sub);
    }

    fn drain(&mut self) -> anyhow::Result<()> {
        SharedEventBus::drain(self);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSub {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl Subscriber for TestSub {
        fn name(&self) -> &str {
            "test"
        }
        fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
            self.events.lock().unwrap_or_else(PoisonError::into_inner).push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn test_publish_and_drain() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut bus = SharedEventBus::new();
        bus.subscribe(Box::new(TestSub {
            events: events.clone(),
        }));

        bus.publish(Event::System(SystemCommand::ReloadSkills))
            .unwrap();
        bus.publish(Event::System(SystemCommand::Shutdown))
            .unwrap();

        assert_eq!(bus.pending(), 2);
        let processed = bus.drain();
        assert_eq!(processed, 2);
        assert_eq!(bus.pending(), 0);
        assert_eq!(events.lock().unwrap_or_else(PoisonError::into_inner).len(), 2);
    }

    #[test]
    fn test_thread_safety() {
        let mut bus = SharedEventBus::new();
        let bus2 = bus.clone();

        let handle = std::thread::spawn(move || {
            let mut b = bus2;
            b.publish(Event::System(SystemCommand::ReloadSkills))
                .unwrap();
        });

        bus.publish(Event::System(SystemCommand::ReloadConfig))
            .unwrap();
        handle.join().unwrap();

        assert_eq!(bus.pending(), 2);
    }
}
