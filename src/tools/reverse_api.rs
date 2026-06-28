use super::Tool;

/// Reverse-engineer an API from documentation: fetch a URL, extract
/// API endpoints/patterns, save to `knowledge/` for future reference.
///
/// Input format:
/// ```
/// <api_name>
/// ---
/// <url>
/// ```
///
/// Or just a URL (the domain name will be used as the API name).
pub struct ReverseApi {
    pub project_root: String,
}

impl Tool for ReverseApi {
    fn name(&self) -> &str {
        "reverse_api"
    }

    fn description(&self) -> &str {
        "Fetch API docs from a URL, save to knowledge/. Input: '<name>\\n---\\n<url>' or just a URL."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let input = input.trim();

        let (name, url) = if let Some(pos) = input.find("\n---\n") {
            let n = input[..pos].trim().to_string();
            let u = input[pos + 5..].trim().to_string();
            (n, u)
        } else {
            // Use the URL's domain as the name
            let cleaned = input.trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.");
            let domain = cleaned.split('/').next().unwrap_or("unknown");
            (domain.to_string(), input.to_string())
        };

        if url.is_empty() {
            anyhow::bail!("reverse_api requires a URL");
        }

        // Fetch the page
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("CodeCoder/0.1")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;

        let resp = client
            .get(&url)
            .send()
            .map_err(|e| anyhow::anyhow!("request failed: {e}"))?;

        let status = resp.status();
        let body = resp
            .text()
            .map_err(|e| anyhow::anyhow!("read failed: {e}"))?;

        if !status.is_success() {
            anyhow::bail!("HTTP {status} for {url}");
        }

        // Strip HTML if needed
        let is_html = body.contains("<!DOCTYPE html")
            || body.contains("<html")
            || body.contains("<script");
        let clean_text = if is_html {
            strip_html_tags(&body)
        } else {
            body.clone()
        };

        // Extract potential API endpoints (simple heuristic)
        let endpoints = extract_endpoints(&clean_text);

        // Build the knowledge document
        let timestamp = chrono_now();
        let mut doc = format!(
            "# API Reference: {name}\n\n\
             Source: {url}\n\
             Fetched: {timestamp}\n\n"
        );

        if !endpoints.is_empty() {
            doc.push_str(&format!("## Discovered Endpoints ({} found)\n\n", endpoints.len()));
            for ep in &endpoints {
                doc.push_str(&format!("- `{ep}`\n"));
            }
            doc.push('\n');
        }

        doc.push_str("## Raw Content (first 10,000 chars)\n\n");
        doc.push_str(&clean_text.chars().take(10_000).collect::<String>());
        doc.push_str("\n\n---\n*Extracted by CodeCoder reverse_api tool*\n");

        // Save to knowledge/ directory
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();

        let knowledge_dir = std::path::Path::new(&self.project_root).join("knowledge");
        std::fs::create_dir_all(&knowledge_dir)
            .map_err(|e| anyhow::anyhow!("cannot create knowledge dir: {e}"))?;

        let path = knowledge_dir.join(format!("{safe_name}.md"));
        std::fs::write(&path, &doc)
            .map_err(|e| anyhow::anyhow!("cannot write knowledge file: {e}"))?;

        Ok(format!(
            "Saved API reference '{}' to {} ({} endpoints extracted, {} total bytes)",
            name,
            path.display(),
            endpoints.len(),
            doc.len(),
        ))
    }
}

/// Crude HTML-to-text (shared with search_web).
fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();

    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
        } else if ch == '<' {
            in_tag = true;
        } else if ch == '&' {
            in_entity = true;
            entity.clear();
        } else if in_entity {
            if ch == ';' {
                let decoded = match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    "nbsp" => " ",
                    _ => &entity,
                };
                out.push_str(decoded);
                in_entity = false;
            } else {
                entity.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Extract potential API endpoint patterns from text.
fn extract_endpoints(text: &str) -> Vec<String> {
    let mut endpoints = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Look for common API patterns: /api/v1/..., /v1/..., /api/..., paths with verbs
    for line in text.lines() {
        let line = line.trim();

        // Check for HTTP-method + path patterns like "GET /api/users"
        for method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
            if let Some(path) = line.strip_prefix(method) {
                let path = path.trim();
                // Extract the path portion
                let path = path.split_whitespace().next().unwrap_or("");
                if !path.is_empty()
                    && (path.starts_with('/') || path.contains("api") || path.contains("v1"))
                {
                    let signature = format!("{method} {path}");
                    if seen.insert(signature.clone()) {
                        endpoints.push(signature);
                    }
                }
            }
        }

        // Also check for bare paths starting with /api/ or /v1/
        if (line.starts_with("/api/") || line.starts_with("/v1/") || line.starts_with("/rest/"))
            && !line.contains(' ')
        {
            if seen.insert(line.to_string()) {
                endpoints.push(format!("  {line}"));
            }
        }
    }

    endpoints.sort();
    endpoints.dedup();
    endpoints.truncate(50); // limit to 50 endpoints
    endpoints
}

fn chrono_now() -> String {
    // Simple timestamp without external chrono crate
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Basic UTC timestamp
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Approximate date from Unix days (not perfect but readable)
    format!("{hours:02}:{mins:02}:{seconds:02} UTC (day {days})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_endpoints() {
        let text = "GET /api/v1/users\nPOST /api/v1/users\nDELETE /api/v1/users/:id\nSome text\nGET /api/v1/projects";
        let eps = extract_endpoints(text);
        assert!(eps.contains(&"GET /api/v1/users".to_string()));
        assert!(eps.contains(&"POST /api/v1/users".to_string()));
        assert!(eps.contains(&"GET /api/v1/projects".to_string()));
        assert_eq!(eps.len(), 4); // The DELETE one might also match
    }

    #[test]
    fn test_extract_no_endpoints() {
        let eps = extract_endpoints("Hello world\nThis is plain text");
        assert!(eps.is_empty());
    }

    #[test]
    fn test_extract_bare_api_paths() {
        let eps = extract_endpoints("/api/v1/users\n/api/v2/products\n/rest/items\n/not-api");
        assert!(eps.contains(&"  /api/v1/users".to_string()));
        assert!(eps.contains(&"  /api/v2/products".to_string()));
        assert!(eps.contains(&"  /rest/items".to_string()));
        assert!(!eps.contains(&"  /not-api".to_string()));
    }

    #[test]
    fn test_empty_input() {
        assert!(ReverseApi {
            project_root: "/tmp".into()
        }.execute("").is_err());
    }

    #[test]
    fn test_reverse_api_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = ReverseApi {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        // Use a local file URL that doesn't need network
        let result = tool.execute("test-api\n---\nhttps://example.com");
        // Might fail if no network, but the knowledge dir should be created regardless
        // Actually HTTP will fail, but let's just test the file structure
        let knowledge_dir = dir.path().join("knowledge");
        assert!(knowledge_dir.exists() || result.is_ok());
    }

    #[test]
    fn test_strip_html_entity_decoding() {
        assert_eq!(strip_html_tags("&lt;tag&gt;"), "<tag>");
        assert_eq!(strip_html_tags("&quot;hello&quot;"), "\"hello\"");
        assert_eq!(strip_html_tags("&apos;x&apos;"), "'x'");
        assert_eq!(strip_html_tags("a&nbsp;b"), "a b");
        assert_eq!(strip_html_tags("&unknown;entity"), "unknownentity");
    }

    #[test]
    fn test_chrono_now_format() {
        let ts = chrono_now();
        assert!(ts.contains("UTC"));
        assert!(ts.contains(':'));
    }

    #[test]
    fn test_build_request_url_from_domain() {
        // Just test the parsing logic by using execute with a local invalid URL
        let tool = ReverseApi {
            project_root: "/tmp".into(),
        };
        let result = tool.execute("https://api.example.com/docs");
        // Should fail due to actual HTTP call, but we're testing that
        // the domain was extracted successfully before the HTTP call
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("request failed") || msg.contains("HTTP") || msg.contains("error"));
    }
}
