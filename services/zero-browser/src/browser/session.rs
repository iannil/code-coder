//! Browser session management.

use crate::error::BrowserError;
use crate::pattern::{RequestSnapshot, ResponseSnapshot};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_headless")]
    pub headless: bool,
    pub proxy: Option<String>,
    #[serde(default = "default_viewport")]
    pub viewport: Viewport,
}

fn default_headless() -> bool {
    true
}

fn default_viewport() -> Viewport {
    Viewport {
        width: 1280,
        height: 720,
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            headless: true,
            proxy: None,
            viewport: default_viewport(),
        }
    }
}

/// Viewport dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Learning,
    Expired,
    Closed,
}

/// Learning filter configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearnFilter {
    /// Only learn from these hosts.
    #[serde(default)]
    pub hosts: Vec<String>,
    /// Only learn paths starting with this prefix.
    pub path_prefix: Option<String>,
    /// Only learn these HTTP methods.
    #[serde(default)]
    pub methods: Vec<String>,
}

/// Browser session.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub config: SessionConfig,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub learn_filter: Option<LearnFilter>,
    pub captured_requests: Vec<(RequestSnapshot, Option<ResponseSnapshot>)>,
}

impl Session {
    /// Create a new session.
    pub fn new(config: SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            config,
            status: SessionStatus::Active,
            created_at: now,
            last_activity: now,
            learn_filter: None,
            captured_requests: Vec::new(),
        }
    }

    /// Check if session has expired.
    pub fn is_expired(&self, timeout_secs: i64) -> bool {
        let elapsed = Utc::now() - self.last_activity;
        elapsed > Duration::seconds(timeout_secs)
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Start learning mode.
    pub fn start_learning(&mut self, filter: LearnFilter) {
        self.status = SessionStatus::Learning;
        self.learn_filter = Some(filter);
        self.captured_requests.clear();
        self.touch();
    }

    /// Stop learning mode and return captured requests.
    pub fn stop_learning(&mut self) -> Vec<(RequestSnapshot, Option<ResponseSnapshot>)> {
        self.status = SessionStatus::Active;
        self.learn_filter = None;
        self.touch();
        std::mem::take(&mut self.captured_requests)
    }

    /// Check if a request should be captured based on filter.
    pub fn should_capture(&self, url: &str, method: &str) -> bool {
        let Some(ref filter) = self.learn_filter else {
            return false;
        };

        // Check host filter
        if !filter.hosts.is_empty() {
            let host_matches = filter.hosts.iter().any(|h| url.contains(h));
            if !host_matches {
                return false;
            }
        }

        // Check path prefix
        if let Some(ref prefix) = filter.path_prefix {
            if let Ok(parsed) = url::Url::parse(url) {
                if !parsed.path().starts_with(prefix) {
                    return false;
                }
            }
        }

        // Check method filter
        if !filter.methods.is_empty() && !filter.methods.contains(&method.to_uppercase()) {
            return false;
        }

        true
    }
}

/// Session manager.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    timeout_secs: i64,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(timeout_secs: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            timeout_secs,
        }
    }

    /// Create a new session.
    pub async fn create(&self, config: SessionConfig) -> String {
        let session = Session::new(config);
        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), session);
        id
    }

    /// Get a session by ID.
    pub async fn get(&self, id: &str) -> Result<Session, BrowserError> {
        let sessions = self.sessions.read().await;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))
    }

    /// Update a session.
    pub async fn update<F>(&self, id: &str, f: F) -> Result<(), BrowserError>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))?;
        f(session);
        Ok(())
    }

    /// Close a session.
    pub async fn close(&self, id: &str) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.write().await;
        sessions
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))
    }

    /// Clean up expired sessions.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| !s.is_expired(self.timeout_secs));
        before - sessions.len()
    }
}

// Manual Clone for Session (needed because of captured_requests)
impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            config: self.config.clone(),
            status: self.status,
            created_at: self.created_at,
            last_activity: self.last_activity,
            learn_filter: self.learn_filter.clone(),
            captured_requests: self.captured_requests.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new(SessionConfig::default());
        assert!(!session.id.is_empty());
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn test_session_learning() {
        let mut session = Session::new(SessionConfig::default());

        session.start_learning(LearnFilter {
            hosts: vec!["api.example.com".to_string()],
            ..Default::default()
        });

        assert_eq!(session.status, SessionStatus::Learning);
        assert!(session.should_capture("https://api.example.com/users", "GET"));
        assert!(!session.should_capture("https://other.com/users", "GET"));
    }

    #[test]
    fn test_session_filter_methods() {
        let mut session = Session::new(SessionConfig::default());
        session.start_learning(LearnFilter {
            methods: vec!["POST".to_string(), "PUT".to_string()],
            ..Default::default()
        });

        assert!(session.should_capture("https://example.com/api", "POST"));
        assert!(!session.should_capture("https://example.com/api", "GET"));
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new(1800);

        let id = manager.create(SessionConfig::default()).await;
        assert!(!id.is_empty());

        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.id, id);

        manager.close(&id).await.unwrap();
        assert!(manager.get(&id).await.is_err());
    }
}
