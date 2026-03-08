//! Extract API patterns from captured network traffic.

use crate::pattern::types::{ApiPattern, AuthPattern, HeaderPattern, RequestSnapshot, ResponseSnapshot};
use std::collections::HashMap;
use url::Url;

/// Extract patterns from captured requests.
pub fn extract_patterns(
    requests: &[(RequestSnapshot, Option<ResponseSnapshot>)],
) -> Vec<ApiPattern> {
    let mut patterns: HashMap<String, ApiPattern> = HashMap::new();

    for (request, response) in requests {
        let Some(pattern) = extract_single_pattern(request, response.as_ref()) else {
            continue;
        };

        // Merge with existing pattern or insert new
        patterns
            .entry(pattern.id.clone())
            .and_modify(|existing| {
                existing.usage_count += 1;
                // Could merge headers, schemas, etc.
            })
            .or_insert(pattern);
    }

    patterns.into_values().collect()
}

/// Extract a pattern from a single request/response pair.
fn extract_single_pattern(
    request: &RequestSnapshot,
    response: Option<&ResponseSnapshot>,
) -> Option<ApiPattern> {
    let url = Url::parse(&request.url).ok()?;
    let host = url.host_str()?;

    // Extract path pattern (replace numeric IDs with {id})
    let path_pattern = extract_path_pattern(url.path());

    let mut pattern = ApiPattern::new(host, &request.method, &path_pattern);

    // Extract authentication
    pattern.auth = extract_auth(&request.headers);

    // Extract required headers (skip common ones)
    pattern.required_headers = extract_required_headers(&request.headers);

    // Extract response schema if JSON
    if let Some(resp) = response {
        if let Some(ref body) = resp.body {
            if is_json_content_type(&resp.headers) {
                pattern.response_schema = extract_json_schema(body);
            }
        }
    }

    // Extract request schema if JSON
    if let Some(ref body) = request.body {
        if is_json_content_type(&request.headers) {
            pattern.request_schema = extract_json_schema(body);
        }
    }

    Some(pattern)
}

/// Convert path to pattern by replacing numeric segments with {id}.
fn extract_path_pattern(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() {
                "{id}".to_string()
            } else if is_uuid(segment) {
                "{id}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Check if a string looks like a UUID.
fn is_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    s.chars().enumerate().all(|(i, c)| {
        if matches!(i, 8 | 13 | 18 | 23) {
            c == '-'
        } else {
            c.is_ascii_hexdigit()
        }
    })
}

/// Extract authentication pattern from headers.
fn extract_auth(headers: &HashMap<String, String>) -> Option<AuthPattern> {
    // Check for Bearer token
    if let Some(auth) = headers.get("authorization").or(headers.get("Authorization")) {
        if auth.to_lowercase().starts_with("bearer ") {
            return Some(AuthPattern::Bearer {
                token_source: "dynamic".to_string(),
            });
        }
    }

    // Check for API key headers
    for key in ["x-api-key", "api-key", "apikey"] {
        if headers.contains_key(key) {
            return Some(AuthPattern::ApiKey {
                header: key.to_string(),
                key_source: "dynamic".to_string(),
            });
        }
    }

    // Check for cookie auth
    if headers.contains_key("cookie") || headers.contains_key("Cookie") {
        return Some(AuthPattern::Cookie {
            names: vec!["session".to_string()],
        });
    }

    None
}

/// Extract required headers (excluding common ones).
fn extract_required_headers(headers: &HashMap<String, String>) -> HashMap<String, HeaderPattern> {
    let skip_headers = [
        "host",
        "user-agent",
        "accept",
        "accept-language",
        "accept-encoding",
        "connection",
        "cookie",
        "authorization",
        "content-length",
        "content-type",
        "origin",
        "referer",
        "sec-",
        "cache-control",
        "pragma",
    ];

    headers
        .iter()
        .filter(|(k, _)| {
            let lower = k.to_lowercase();
            !skip_headers.iter().any(|s| lower.starts_with(s))
        })
        .map(|(k, v)| {
            (
                k.clone(),
                HeaderPattern::Fixed {
                    value: v.clone(),
                },
            )
        })
        .collect()
}

/// Check if content type is JSON.
fn is_json_content_type(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .any(|(k, v)| k.to_lowercase() == "content-type" && v.contains("application/json"))
}

/// Extract a simple JSON schema from body bytes.
fn extract_json_schema(body: &[u8]) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    Some(infer_schema(&value))
}

/// Infer JSON schema from a value.
fn infer_schema(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({"type": "null"}),
        serde_json::Value::Bool(_) => serde_json::json!({"type": "boolean"}),
        serde_json::Value::Number(_) => serde_json::json!({"type": "number"}),
        serde_json::Value::String(_) => serde_json::json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            let items = arr.first().map(infer_schema).unwrap_or(serde_json::json!({}));
            serde_json::json!({"type": "array", "items": items})
        }
        serde_json::Value::Object(obj) => {
            let properties: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), infer_schema(v)))
                .collect();
            serde_json::json!({"type": "object", "properties": properties})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_path_pattern() {
        assert_eq!(extract_path_pattern("/users/123"), "/users/{id}");
        assert_eq!(extract_path_pattern("/api/v1/posts"), "/api/v1/posts");
        assert_eq!(
            extract_path_pattern("/users/123/posts/456"),
            "/users/{id}/posts/{id}"
        );
    }

    #[test]
    fn test_extract_auth_bearer() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer abc123".to_string());

        let auth = extract_auth(&headers);
        assert!(matches!(auth, Some(AuthPattern::Bearer { .. })));
    }

    #[test]
    fn test_extract_auth_api_key() {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "secret".to_string());

        let auth = extract_auth(&headers);
        assert!(matches!(auth, Some(AuthPattern::ApiKey { .. })));
    }

    #[test]
    fn test_extract_patterns() {
        let request = RequestSnapshot {
            url: "https://api.example.com/users/123".to_string(),
            method: "GET".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Authorization".to_string(), "Bearer token".to_string());
                h
            },
            body: None,
            timestamp: Utc::now(),
        };

        let response = ResponseSnapshot {
            status: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: Some(br#"{"id": 123, "name": "Alice"}"#.to_vec()),
            duration_ms: 50,
        };

        let patterns = extract_patterns(&[(request, Some(response))]);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].path_pattern, "/users/{id}");
        assert!(patterns[0].auth.is_some());
        assert!(patterns[0].response_schema.is_some());
    }

    #[test]
    fn test_infer_schema() {
        let value = serde_json::json!({"name": "Alice", "age": 30});
        let schema = infer_schema(&value);

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "number");
    }
}
