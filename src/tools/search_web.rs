use super::Tool;

/// Fetch a URL and return its text content.
///
/// Uses `reqwest::blocking` (native Rust HTTP, no shelling out to curl).
pub struct SearchWeb;

impl Tool for SearchWeb {
    fn name(&self) -> &str {
        "search_web"
    }

    fn description(&self) -> &str {
        "Fetch a URL and return its content as text. Input: URL."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let url = input.trim();
        if url.is_empty() {
            anyhow::bail!("search_web requires a URL");
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("CodeCoder/0.1")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create HTTP client: {e}"))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} for {url}");
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response
            .text()
            .map_err(|e| anyhow::anyhow!("failed to read response body: {e}"))?;

        // If HTML, strip tags for readability
        let text = if content_type.contains("text/html") {
            strip_html_tags(&body)
        } else {
            body
        };

        // Truncate to avoid blowing context
        const MAX_LEN: usize = 50_000;
        if text.len() > MAX_LEN {
            Ok(format!(
                "{} … [truncated, {} total bytes]",
                &text[..MAX_LEN],
                text.len()
            ))
        } else {
            Ok(text)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Hello <b>world</b> &amp; friends</p>";
        assert_eq!(strip_html_tags(html), "Hello world & friends");
    }

    #[test]
    fn test_strip_html_all_entities() {
        assert_eq!(strip_html_tags("&lt;code&gt;"), "<code>");
        assert_eq!(strip_html_tags("&quot;str&quot;"), "\"str\"");
        assert_eq!(strip_html_tags("&apos;x&apos;"), "'x'");
        assert_eq!(strip_html_tags("a&nbsp;b"), "a b");
        assert_eq!(strip_html_tags("&unknown;val"), "unknownval");
        // incomplete entity (no semicolon) — characters are consumed but not output
        assert_eq!(strip_html_tags("text&amp"), "text");
    }

    #[test]
    fn test_strip_html_no_tags() {
        assert_eq!(strip_html_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_html_numeric_entities() {
        // Numeric entities like &#39; and &#x27; should be passed through
        // since the simple entity handler doesn't decode them
        let result = strip_html_tags("&#39;quote&#x27;");
        assert_eq!(result, "''"); // &#39; has entity "39" which doesn't match any known entity
    }

    #[test]
    fn test_strip_html_self_closing_tag() {
        assert_eq!(strip_html_tags("<br/>text"), "text");
    }

    #[test]
    fn test_strip_html_mixed_content() {
        let html = "<div><p>Hello <b>world</b></p><ul><li>item</li></ul></div>";
        let result = strip_html_tags(html);
        assert_eq!(result, "Hello worlditem");
    }

    #[test]
    fn test_empty_url() {
        assert!(SearchWeb.execute("").is_err());
    }
}
