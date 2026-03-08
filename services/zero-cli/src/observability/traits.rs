use std::time::Duration;

/// Events the observer can record
#[derive(Debug, Clone)]
pub enum ObserverEvent {
    AgentStart {
        provider: String,
        model: String,
    },
    AgentEnd {
        duration: Duration,
        tokens_used: Option<u64>,
    },
    ToolCall {
        tool: String,
        duration: Duration,
        success: bool,
    },
    ChannelMessage {
        channel: String,
        direction: String,
    },
    HeartbeatTick,
    Error {
        component: String,
        message: String,
    },
    // Heartbeat monitor events
    ServiceRestart {
        service_id: String,
    },
    Alert {
        service_id: String,
        channels: Vec<String>,
    },
    Escalation {
        service_id: String,
        escalate_to: Vec<String>,
    },
    HealthChange {
        service_id: String,
        previous_status: String,
        current_status: String,
    },
}

/// Numeric metrics
#[derive(Debug, Clone)]
pub enum ObserverMetric {
    RequestLatency(Duration),
    TokensUsed(u64),
    ActiveSessions(u64),
    QueueDepth(u64),
}

/// Core observability trait — implement for any backend
pub trait Observer: Send + Sync {
    /// Record a discrete event
    fn record_event(&self, event: &ObserverEvent);

    /// Record a numeric metric
    fn record_metric(&self, metric: &ObserverMetric);

    /// Flush any buffered data (no-op for most backends)
    fn flush(&self) {}

    /// Human-readable name of this observer
    fn name(&self) -> &str;
}
