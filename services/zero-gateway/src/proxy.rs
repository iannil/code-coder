//! Proxy module for Zero Gateway.
//!
//! Handles proxying requests to the CodeCoder API.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::Response,
};
use reqwest::Client;
use std::sync::Arc;

/// Proxy state.
#[derive(Clone)]
pub struct ProxyState {
    pub client: Client,
    pub target_url: Arc<String>,
}

impl ProxyState {
    /// Create a new proxy state.
    pub fn new(target_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            target_url: Arc::new(target_url.into()),
        }
    }
}

/// Proxy a request to the CodeCoder API.
pub async fn proxy_request(
    State(state): State<ProxyState>,
    request: Request,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri();
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    // Strip /api/v1/proxy prefix if present
    let target_path = path.strip_prefix("/api/v1/proxy").unwrap_or(path);
    let target_url = format!("{}{}", state.target_url, target_path);

    tracing::debug!(
        method = %method,
        target_url = %target_url,
        "Proxying request"
    );

    // Build the proxy request
    let mut builder = state.client.request(method, &target_url);

    // Copy headers (except host and content-length which will be set automatically)
    for (name, value) in request.headers() {
        if name != header::HOST && name != header::CONTENT_LENGTH {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }
    }

    // Forward the body
    let body_bytes = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes.to_vec());
    }

    // Send the request
    let response = builder.send().await.map_err(|e| {
        tracing::error!(error = %e, "Proxy request failed");
        StatusCode::BAD_GATEWAY
    })?;

    // Build the response
    let status = StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut response_builder = Response::builder().status(status);

    // Copy response headers
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            response_builder = response_builder.header(name.as_str(), v);
        }
    }

    // Get response body
    let body_bytes = response.bytes().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to read proxy response body");
        StatusCode::BAD_GATEWAY
    })?;

    response_builder
        .body(Body::from(body_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_state_creation() {
        let state = ProxyState::new("http://localhost:4400");
        assert_eq!(*state.target_url, "http://localhost:4400");
    }
}
