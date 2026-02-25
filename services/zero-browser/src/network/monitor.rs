//! Network monitoring via CDP.

use crate::browser::SessionManager;
use crate::error::BrowserError;
use crate::pattern::{RequestSnapshot, ResponseSnapshot};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network request entry being tracked.
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub request_id: String,
    pub snapshot: RequestSnapshot,
    pub start_time: std::time::Instant,
}

/// Network monitor for a browser session.
#[derive(Debug)]
pub struct NetworkMonitor {
    session_id: String,
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    session_manager: SessionManager,
}

impl NetworkMonitor {
    /// Create a new network monitor.
    pub fn new(session_id: String, session_manager: SessionManager) -> Self {
        Self {
            session_id,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            session_manager,
        }
    }

    /// Handle a network request event from CDP.
    pub async fn on_request(
        &self,
        request_id: &str,
        url: &str,
        method: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<(), BrowserError> {
        // Check if we should capture this request
        let session = self.session_manager.get(&self.session_id).await?;
        if !session.should_capture(url, method) {
            return Ok(());
        }

        let snapshot = RequestSnapshot {
            url: url.to_string(),
            method: method.to_string(),
            headers,
            body,
            timestamp: Utc::now(),
        };

        let pending = PendingRequest {
            request_id: request_id.to_string(),
            snapshot,
            start_time: std::time::Instant::now(),
        };

        self.pending_requests
            .write()
            .await
            .insert(request_id.to_string(), pending);

        Ok(())
    }

    /// Handle a network response event from CDP.
    pub async fn on_response(
        &self,
        request_id: &str,
        status: u16,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<(), BrowserError> {
        let pending = {
            let mut pending_requests = self.pending_requests.write().await;
            pending_requests.remove(request_id)
        };

        let Some(pending) = pending else {
            // Request wasn't being tracked
            return Ok(());
        };

        let duration_ms = pending.start_time.elapsed().as_millis() as u64;

        let response = ResponseSnapshot {
            status,
            headers,
            body,
            duration_ms,
        };

        // Store in session
        self.session_manager
            .update(&self.session_id, |session| {
                session
                    .captured_requests
                    .push((pending.snapshot, Some(response)));
            })
            .await?;

        Ok(())
    }

    /// Get count of pending requests.
    pub async fn pending_count(&self) -> usize {
        self.pending_requests.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::{LearnFilter, SessionConfig};

    #[tokio::test]
    async fn test_network_monitor_captures_matching_requests() {
        let manager = SessionManager::new(1800);
        let session_id = manager.create(SessionConfig::default()).await;

        // Start learning
        manager
            .update(&session_id, |s| {
                s.start_learning(LearnFilter {
                    hosts: vec!["api.example.com".to_string()],
                    ..Default::default()
                });
            })
            .await
            .unwrap();

        let monitor = NetworkMonitor::new(session_id.clone(), manager.clone());

        // Send matching request
        monitor
            .on_request(
                "req1",
                "https://api.example.com/users",
                "GET",
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(monitor.pending_count().await, 1);

        // Send response
        monitor
            .on_response("req1", 200, HashMap::new(), None)
            .await
            .unwrap();

        assert_eq!(monitor.pending_count().await, 0);

        // Verify captured
        let session = manager.get(&session_id).await.unwrap();
        assert_eq!(session.captured_requests.len(), 1);
    }

    #[tokio::test]
    async fn test_network_monitor_ignores_non_matching() {
        let manager = SessionManager::new(1800);
        let session_id = manager.create(SessionConfig::default()).await;

        manager
            .update(&session_id, |s| {
                s.start_learning(LearnFilter {
                    hosts: vec!["api.example.com".to_string()],
                    ..Default::default()
                });
            })
            .await
            .unwrap();

        let monitor = NetworkMonitor::new(session_id.clone(), manager.clone());

        // Send non-matching request
        monitor
            .on_request(
                "req1",
                "https://other.com/users",
                "GET",
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        // Should not be tracked
        assert_eq!(monitor.pending_count().await, 0);
    }
}
