//! Axum tracing middleware for distributed tracing.
//!
//! This middleware extracts trace context from incoming HTTP headers,
//! logs request/response events, and propagates trace IDs to downstream services.
//!
//! # Usage
//!
//! ```ignore
//! use zero_common::tracing_middleware::TracingLayer;
//!
//! let app = Router::new()
//!     .route("/api/health", get(health))
//!     .layer(TracingLayer::new("zero-gateway"));
//! ```

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::logging::{LifecycleEventType, RequestContext};

/// Tracing middleware layer for Axum.
#[derive(Clone)]
#[allow(dead_code)]
pub struct TracingLayer {
    service_name: String,
}

impl TracingLayer {
    /// Create a new tracing layer with the given service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

/// Extract trace context from request and log HTTP request/response events.
pub async fn tracing_middleware(
    service_name: String,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Extract trace context from headers
    let ctx = RequestContext::from_headers(request.headers(), &service_name);

    // Log HTTP request
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let path = request.uri().path().to_string();

    ctx.log_event(
        LifecycleEventType::HttpRequest,
        serde_json::json!({
            "method": method,
            "path": path,
            "uri": uri,
        }),
    );

    // Process the request
    let mut response = next.run(request).await;

    // Calculate duration
    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Add trace headers to response
    ctx.to_headers(response.headers_mut());

    // Log HTTP response
    ctx.log_event(
        LifecycleEventType::HttpResponse,
        serde_json::json!({
            "method": method,
            "path": path,
            "status": status,
            "duration_ms": duration_ms,
        }),
    );

    response
}

/// Create a tracing middleware function for use with `axum::middleware::from_fn_with_state`.
pub fn make_tracing_middleware(
    service_name: String,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone + Send + 'static {
    move |request: Request, next: Next| {
        let service = service_name.clone();
        Box::pin(async move {
            tracing_middleware(service, request, next).await
        })
    }
}

/// Extension trait for adding tracing to Axum routers.
pub trait TracingExt {
    /// Add tracing middleware to this router.
    fn with_tracing(self, service_name: impl Into<String>) -> Self;
}

impl TracingExt for axum::Router {
    fn with_tracing(self, service_name: impl Into<String>) -> Self {
        let service = service_name.into();
        self.layer(axum::middleware::from_fn(move |req, next| {
            let svc = service.clone();
            async move { tracing_middleware(svc, req, next).await }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use axum::body::Body;
    use http::{Request as HttpRequest, StatusCode};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_tracing_middleware_adds_headers() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .with_tracing("test-service");

        let request = HttpRequest::builder()
            .uri("/test")
            .header("X-Trace-Id", "test-trace-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check that trace headers are propagated
        assert!(response.headers().contains_key("X-Trace-Id"));
        assert!(response.headers().contains_key("X-Span-Id"));
    }

    #[tokio::test]
    async fn test_tracing_middleware_generates_trace_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .with_tracing("test-service");

        let request = HttpRequest::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Should generate trace ID if not provided
        assert!(response.headers().contains_key("X-Trace-Id"));
        assert!(response.headers().contains_key("X-Span-Id"));
    }
}
