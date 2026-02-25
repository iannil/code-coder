//! API replay execution.

use crate::error::BrowserError;
use crate::pattern::{ApiPattern, AuthPattern, HeaderPattern};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Replay request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayParams {
    /// Pattern ID to replay.
    pub pattern_id: String,
    /// Path parameter substitutions.
    #[serde(default)]
    pub path_params: HashMap<String, String>,
    /// Query parameters.
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Request body.
    pub body: Option<serde_json::Value>,
    /// Authentication override.
    pub auth: Option<ReplayAuth>,
}

/// Authentication for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReplayAuth {
    Bearer { token: String },
    ApiKey { header: String, value: String },
    Cookie { value: String },
}

/// Replay response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
    pub duration_ms: u64,
}

/// API replay executor.
pub struct ReplayExecutor {
    client: Client,
}

impl ReplayExecutor {
    /// Create a new replay executor.
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Execute a replay request.
    pub async fn execute(
        &self,
        pattern: &ApiPattern,
        params: &ReplayParams,
    ) -> Result<ReplayResponse, BrowserError> {
        let start = std::time::Instant::now();

        // Build URL
        let url = self.build_url(pattern, params)?;

        // Build request
        let mut request = match pattern.method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            method => {
                return Err(BrowserError::ReplayFailed {
                    pattern_id: pattern.id.clone(),
                    reason: format!("Unsupported method: {}", method),
                })
            }
        };

        // Add headers from pattern
        for (name, header_pattern) in &pattern.required_headers {
            if let HeaderPattern::Fixed { value } = header_pattern {
                request = request.header(name, value);
            }
        }

        // Add authentication
        request = self.apply_auth(request, pattern, params)?;

        // Add body
        if let Some(ref body) = params.body {
            request = request.json(body);
        }

        // Execute
        let response = request.send().await.map_err(|e| BrowserError::ReplayFailed {
            pattern_id: pattern.id.clone(),
            reason: e.to_string(),
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();

        // Collect headers
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Parse body
        let body = response.json().await.unwrap_or(serde_json::Value::Null);

        Ok(ReplayResponse {
            status,
            headers,
            body,
            duration_ms,
        })
    }

    /// Build the full URL with path and query parameters.
    fn build_url(&self, pattern: &ApiPattern, params: &ReplayParams) -> Result<String, BrowserError> {
        // Substitute path parameters
        let mut path = pattern.path_pattern.clone();
        for (key, value) in &params.path_params {
            path = path.replace(&format!("{{{}}}", key), value);
        }

        // Check for unsubstituted placeholders
        if path.contains('{') {
            return Err(BrowserError::ReplayFailed {
                pattern_id: pattern.id.clone(),
                reason: format!("Unsubstituted path parameters in: {}", path),
            });
        }

        // Build full URL
        let mut url = format!("https://{}{}", pattern.host, path);

        // Add query parameters
        if !params.query_params.is_empty() {
            let query: String = params
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query);
        }

        Ok(url)
    }

    /// Apply authentication to the request.
    fn apply_auth(
        &self,
        mut request: reqwest::RequestBuilder,
        pattern: &ApiPattern,
        params: &ReplayParams,
    ) -> Result<reqwest::RequestBuilder, BrowserError> {
        // Use override auth if provided
        if let Some(ref auth) = params.auth {
            match auth {
                ReplayAuth::Bearer { token } => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                ReplayAuth::ApiKey { header, value } => {
                    request = request.header(header, value);
                }
                ReplayAuth::Cookie { value } => {
                    request = request.header("Cookie", value);
                }
            }
            return Ok(request);
        }

        // Use pattern auth (would need credential lookup in real implementation)
        if let Some(ref auth) = pattern.auth {
            match auth {
                AuthPattern::Bearer { token_source } => {
                    // In real implementation, resolve token_source
                    if token_source != "dynamic" {
                        return Err(BrowserError::ReplayFailed {
                            pattern_id: pattern.id.clone(),
                            reason: "Auth token required but not provided".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(request)
    }
}

impl Default for ReplayExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_simple() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            auth: None,
        };

        let url = executor.build_url(&pattern, &params).unwrap();
        assert_eq!(url, "https://api.example.com/users");
    }

    #[test]
    fn test_build_url_with_params() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}/posts/{post_id}");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: {
                let mut m = HashMap::new();
                m.insert("id".to_string(), "123".to_string());
                m.insert("post_id".to_string(), "456".to_string());
                m
            },
            query_params: {
                let mut m = HashMap::new();
                m.insert("page".to_string(), "1".to_string());
                m
            },
            body: None,
            auth: None,
        };

        let url = executor.build_url(&pattern, &params).unwrap();
        assert_eq!(url, "https://api.example.com/users/123/posts/456?page=1");
    }

    #[test]
    fn test_build_url_missing_param() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            auth: None,
        };

        let result = executor.build_url(&pattern, &params);
        assert!(result.is_err());
    }
}
