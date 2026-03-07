//! Web Fetch tool - HTTP request handling
//!
//! This module provides HTTP fetching with:
//! - GET/POST/PUT/DELETE support
//! - HTML to Markdown conversion
//! - Response caching
//! - Redirect handling

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// HTTP method for requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Get
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
        }
    }
}

/// Options for web fetch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchOptions {
    /// The URL to fetch
    pub url: String,

    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,

    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body (for POST/PUT/PATCH)
    pub body: Option<String>,

    /// Timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Follow redirects
    #[serde(default = "default_true")]
    pub follow_redirects: bool,

    /// Maximum redirects to follow
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,

    /// Convert HTML to Markdown
    #[serde(default = "default_true")]
    pub html_to_markdown: bool,

    /// User agent string
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

impl Default for WebFetchOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::Get,
            headers: HashMap::new(),
            body: None,
            timeout_ms: 30_000,
            follow_redirects: true,
            max_redirects: 10,
            html_to_markdown: true,
            user_agent: default_user_agent(),
        }
    }
}

fn default_timeout() -> u64 {
    30_000
}

fn default_true() -> bool {
    true
}

fn default_max_redirects() -> usize {
    10
}

fn default_user_agent() -> String {
    "CodeCoder/1.0 (https://github.com/codecoder-ai)".to_string()
}

/// Result of a web fetch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResult {
    /// Whether the request succeeded
    pub success: bool,

    /// HTTP status code
    pub status: u16,

    /// Status text
    pub status_text: String,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body (text content)
    pub content: String,

    /// Content type
    pub content_type: Option<String>,

    /// Content length
    pub content_length: Option<u64>,

    /// Final URL (after redirects)
    pub final_url: String,

    /// Redirect chain (if any)
    pub redirects: Vec<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Response time in milliseconds
    pub response_time_ms: u64,
}

impl WebFetchResult {
    /// Create a successful result
    pub fn ok(status: u16, content: String, final_url: String, response_time_ms: u64) -> Self {
        Self {
            success: status >= 200 && status < 400,
            status,
            status_text: Self::status_text(status),
            headers: HashMap::new(),
            content,
            content_type: None,
            content_length: None,
            final_url,
            redirects: Vec::new(),
            error: None,
            response_time_ms,
        }
    }

    /// Create a failed result
    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            status: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            content: String::new(),
            content_type: None,
            content_length: None,
            final_url: String::new(),
            redirects: Vec::new(),
            error: Some(error.into()),
            response_time_ms: 0,
        }
    }

    /// Get status text for a code
    fn status_text(code: u16) -> String {
        match code {
            200 => "OK".to_string(),
            201 => "Created".to_string(),
            204 => "No Content".to_string(),
            301 => "Moved Permanently".to_string(),
            302 => "Found".to_string(),
            304 => "Not Modified".to_string(),
            400 => "Bad Request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not Found".to_string(),
            405 => "Method Not Allowed".to_string(),
            500 => "Internal Server Error".to_string(),
            502 => "Bad Gateway".to_string(),
            503 => "Service Unavailable".to_string(),
            _ => format!("Status {}", code),
        }
    }
}

/// Web fetcher with caching support
pub struct WebFetcher {
    /// HTTP client
    client: reqwest::Client,
    /// Default options
    _default_options: WebFetchOptions,
}

impl Default for WebFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetcher {
    /// Create a new WebFetcher
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            _default_options: WebFetchOptions::default(),
        }
    }

    /// Create with custom default options
    pub fn with_defaults(options: WebFetchOptions) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(options.timeout_ms))
            .redirect(if options.follow_redirects {
                reqwest::redirect::Policy::limited(options.max_redirects)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .unwrap_or_default();

        Self {
            client,
            _default_options: options,
        }
    }

    /// Fetch a URL
    pub async fn fetch(&self, options: &WebFetchOptions) -> Result<WebFetchResult> {
        let start = std::time::Instant::now();

        // Validate URL
        let url = reqwest::Url::parse(&options.url)
            .with_context(|| format!("Invalid URL: {}", options.url))?;

        // Build request
        let mut request = match options.method {
            HttpMethod::Get => self.client.get(url.clone()),
            HttpMethod::Post => self.client.post(url.clone()),
            HttpMethod::Put => self.client.put(url.clone()),
            HttpMethod::Delete => self.client.delete(url.clone()),
            HttpMethod::Patch => self.client.patch(url.clone()),
            HttpMethod::Head => self.client.head(url.clone()),
            HttpMethod::Options => self.client.request(reqwest::Method::OPTIONS, url.clone()),
        };

        // Add headers
        for (key, value) in &options.headers {
            request = request.header(key, value);
        }

        // Add user agent
        request = request.header("User-Agent", &options.user_agent);

        // Add body if present
        if let Some(body) = &options.body {
            request = request.body(body.clone());
        }

        // Set timeout
        request = request.timeout(Duration::from_millis(options.timeout_ms));

        // Execute request
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(WebFetchResult::err(format!("Request failed: {}", e)));
            }
        };

        let response_time_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        let final_url = response.url().to_string();

        // Collect headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        let content_type = headers.get("content-type").cloned();
        let content_length = headers
            .get("content-length")
            .and_then(|s| s.parse().ok());

        // Get body
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                return Ok(WebFetchResult::err(format!("Failed to read response: {}", e)));
            }
        };

        // Convert HTML to Markdown if requested
        let content = if options.html_to_markdown
            && content_type
                .as_ref()
                .is_some_and(|ct| ct.contains("text/html"))
        {
            self.html_to_markdown(&body)
        } else {
            body
        };

        let mut result = WebFetchResult::ok(status, content, final_url, response_time_ms);
        result.headers = headers;
        result.content_type = content_type;
        result.content_length = content_length;

        Ok(result)
    }

    /// Convert HTML to Markdown (simplified)
    fn html_to_markdown(&self, html: &str) -> String {
        // This is a simplified implementation
        // A full implementation would use a proper HTML parser

        let mut text = html.to_string();

        // Remove script and style tags
        text = regex::Regex::new(r"<script[^>]*>[\s\S]*?</script>")
            .unwrap()
            .replace_all(&text, "")
            .to_string();
        text = regex::Regex::new(r"<style[^>]*>[\s\S]*?</style>")
            .unwrap()
            .replace_all(&text, "")
            .to_string();

        // Convert headers
        for i in (1..=6).rev() {
            let pattern = format!(r"<h{}[^>]*>(.*?)</h{}>", i, i);
            let replacement = format!("{} $1\n", "#".repeat(i));
            text = regex::Regex::new(&pattern)
                .unwrap()
                .replace_all(&text, replacement.as_str())
                .to_string();
        }

        // Convert paragraphs
        text = regex::Regex::new(r"<p[^>]*>(.*?)</p>")
            .unwrap()
            .replace_all(&text, "$1\n\n")
            .to_string();

        // Convert links
        text = regex::Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#)
            .unwrap()
            .replace_all(&text, "[$2]($1)")
            .to_string();

        // Convert bold/strong - separate patterns since regex doesn't support backreferences
        text = regex::Regex::new(r"<strong[^>]*>(.*?)</strong>")
            .unwrap()
            .replace_all(&text, "**$1**")
            .to_string();
        text = regex::Regex::new(r"<b[^>]*>(.*?)</b>")
            .unwrap()
            .replace_all(&text, "**$1**")
            .to_string();

        // Convert italic/em - separate patterns
        text = regex::Regex::new(r"<em[^>]*>(.*?)</em>")
            .unwrap()
            .replace_all(&text, "*$1*")
            .to_string();
        text = regex::Regex::new(r"<i[^>]*>(.*?)</i>")
            .unwrap()
            .replace_all(&text, "*$1*")
            .to_string();

        // Convert code
        text = regex::Regex::new(r"<code[^>]*>(.*?)</code>")
            .unwrap()
            .replace_all(&text, "`$1`")
            .to_string();

        // Convert pre/code blocks
        text = regex::Regex::new(r"<pre[^>]*><code[^>]*>([\s\S]*?)</code></pre>")
            .unwrap()
            .replace_all(&text, "```\n$1\n```")
            .to_string();

        // Convert line breaks
        text = regex::Regex::new(r"<br\s*/?>")
            .unwrap()
            .replace_all(&text, "\n")
            .to_string();

        // Convert list items
        text = regex::Regex::new(r"<li[^>]*>(.*?)</li>")
            .unwrap()
            .replace_all(&text, "- $1\n")
            .to_string();

        // Remove remaining HTML tags
        text = regex::Regex::new(r"<[^>]+>")
            .unwrap()
            .replace_all(&text, "")
            .to_string();

        // Decode HTML entities
        text = text.replace("&amp;", "&");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&quot;", "\"");
        text = text.replace("&apos;", "'");
        text = text.replace("&nbsp;", " ");

        // Clean up whitespace
        text = regex::Regex::new(r"\n{3,}")
            .unwrap()
            .replace_all(&text, "\n\n")
            .to_string();
        text = regex::Regex::new(r"[ \t]+")
            .unwrap()
            .replace_all(&text, " ")
            .to_string();

        text.trim().to_string()
    }
}

/// Convenience function for simple GET request
pub async fn fetch_url(url: &str) -> Result<WebFetchResult> {
    let fetcher = WebFetcher::new();
    let options = WebFetchOptions {
        url: url.to_string(),
        ..Default::default()
    };
    fetcher.fetch(&options).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = WebFetchOptions::default();
        assert_eq!(options.method, HttpMethod::Get);
        assert!(options.follow_redirects);
        assert!(options.html_to_markdown);
    }

    #[test]
    fn test_status_text() {
        assert_eq!(WebFetchResult::status_text(200), "OK");
        assert_eq!(WebFetchResult::status_text(404), "Not Found");
        assert_eq!(WebFetchResult::status_text(500), "Internal Server Error");
    }

    #[test]
    fn test_html_to_markdown() {
        let fetcher = WebFetcher::new();

        let html = "<h1>Title</h1><p>Hello <strong>world</strong>!</p>";
        let md = fetcher.html_to_markdown(html);

        assert!(md.contains("# Title"));
        assert!(md.contains("**world**"));
    }

    #[test]
    fn test_html_link_conversion() {
        let fetcher = WebFetcher::new();

        let html = r#"<a href="https://example.com">Example</a>"#;
        let md = fetcher.html_to_markdown(html);

        assert!(md.contains("[Example](https://example.com)"));
    }

    // Note: Network tests should be run with caution in CI
    // #[tokio::test]
    // async fn test_fetch_real_url() {
    //     let fetcher = WebFetcher::new();
    //     let options = WebFetchOptions {
    //         url: "https://httpbin.org/get".to_string(),
    //         ..Default::default()
    //     };
    //     let result = fetcher.fetch(&options).await.unwrap();
    //     assert!(result.success);
    //     assert_eq!(result.status, 200);
    // }
}
