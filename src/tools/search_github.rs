use super::Tool;

/// Search GitHub repositories and code via the GitHub API.
///
/// Input format: "repos:<query>" for repository search,
/// "code:<query>" for code search, or just "<query>" (defaults to repos).
pub struct SearchGitHub;

impl Tool for SearchGitHub {
    fn name(&self) -> &str {
        "search_github"
    }

    fn description(&self) -> &str {
        "Search GitHub. Prefix with 'repos:' or 'code:'. Input: 'repos:rust http server'"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let input = input.trim();
        if input.is_empty() {
            anyhow::bail!("search_github requires a query");
        }

        let (mode, query) = if let Some(q) = input.strip_prefix("repos:") {
            ("repos", q.trim())
        } else if let Some(q) = input.strip_prefix("code:") {
            ("code", q.trim())
        } else {
            ("repos", input)
        };

        let api_url = match mode {
            "repos" => format!(
                "https://api.github.com/search/repositories?q={}&per_page=10&sort=stars",
                urlencode(query)
            ),
            "code" => format!(
                "https://api.github.com/search/code?q={}&per_page=10",
                urlencode(query)
            ),
            _ => unreachable!(),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("CodeCoder/0.1")
            .build()
            .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;

        // Optional auth for higher rate limits
        let req = client.get(&api_url);
        let req = if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            req.header("Authorization", format!("Bearer {token}"))
        } else {
            req
        };

        let resp = req
            .send()
            .map_err(|e| anyhow::anyhow!("GitHub API request failed: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .map_err(|e| anyhow::anyhow!("failed to parse GitHub response: {e}"))?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("GitHub API {status}: {msg}");
        }

        match mode {
            "repos" => format_repos(&body),
            "code" => format_code(&body),
            _ => unreachable!(),
        }
    }
}

fn format_repos(json: &serde_json::Value) -> anyhow::Result<String> {
    let total = json["total_count"].as_i64().unwrap_or(0);
    let items = json["items"].as_array().ok_or_else(|| anyhow::anyhow!("no items in response"))?;

    let mut result = format!("GitHub repositories ({} total, showing {}):\n\n", total, items.len());

    for item in items {
        let name = item["full_name"].as_str().unwrap_or("?");
        let desc = item["description"].as_str().unwrap_or("");
        let stars = item["stargazers_count"].as_i64().unwrap_or(0);
        let lang = item["language"].as_str().unwrap_or("?");
        let url = item["html_url"].as_str().unwrap_or("?");
        result.push_str(&format!("⭐ {stars:>5}  {name:<30}  [{lang}]\n"));
        if !desc.is_empty() {
            result.push_str(&format!("      {desc}\n"));
        }
        result.push_str(&format!("      {url}\n\n"));
    }

    Ok(result)
}

fn format_code(json: &serde_json::Value) -> anyhow::Result<String> {
    let total = json["total_count"].as_i64().unwrap_or(0);
    let items = json["items"].as_array().ok_or_else(|| anyhow::anyhow!("no items in response"))?;

    let mut result = format!("GitHub code results ({} total, showing {}):\n\n", total, items.len());

    for item in items {
        let _name = item["name"].as_str().unwrap_or("?");
        let repo = item["repository"]["full_name"].as_str().unwrap_or("?");
        let path = item["path"].as_str().unwrap_or("?");
        let url = item["html_url"].as_str().unwrap_or("?");
        result.push_str(&format!("  {repo}/{path}\n"));
        result.push_str(&format!("    {url}\n\n"));
    }

    Ok(result)
}

fn urlencode(s: &str) -> String {
    s.split(' ')
        .map(|part| urlencoding(part))
        .collect::<Vec<_>>()
        .join("+")
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode() {
        assert_eq!(urlencode("hello world"), "hello+world");
        assert_eq!(urlencode("rust lang"), "rust+lang");
    }

    #[test]
    fn test_empty_input() {
        assert!(SearchGitHub.execute("").is_err());
    }

    #[test]
    fn test_urlencoding_basic() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("a b"), "a%20b");
        assert_eq!(urlencoding("foo/bar"), "foo%2Fbar");
    }

    #[test]
    fn test_urlencoding_special_chars() {
        assert_eq!(urlencoding("a+b"), "a%2Bb");
        assert_eq!(urlencoding("100%"), "100%25");
    }

    #[test]
    fn test_format_repos_basic() {
        let json = serde_json::json!({
            "total_count": 42,
            "items": [
                {
                    "full_name": "user/repo",
                    "description": "A test repo",
                    "stargazers_count": 100,
                    "language": "Rust",
                    "html_url": "https://github.com/user/repo"
                }
            ]
        });
        let result = format_repos(&json).unwrap();
        assert!(result.contains("user/repo"));
        assert!(result.contains("100"));
        assert!(result.contains("A test repo"));
        assert!(result.contains("Rust"));
    }

    #[test]
    fn test_format_repos_no_items() {
        let json = serde_json::json!({"total_count": 0});
        let result = format_repos(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_code_basic() {
        let json = serde_json::json!({
            "total_count": 5,
            "items": [
                {
                    "name": "main.rs",
                    "repository": {"full_name": "user/repo"},
                    "path": "src/main.rs",
                    "html_url": "https://github.com/user/repo/src/main.rs"
                }
            ]
        });
        let result = format_code(&json).unwrap();
        assert!(result.contains("user/repo/src/main.rs"));
    }

    #[test]
    fn test_format_code_no_items() {
        let json = serde_json::json!({"total_count": 0});
        let result = format_code(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_github_name_and_description() {
        let tool = SearchGitHub;
        assert_eq!(tool.name(), "search_github");
        assert!(tool.description().contains("GitHub"));
    }

    #[test]
    fn test_search_github_code_mode_invalid() {
        let tool = SearchGitHub;
        let result = tool.execute("code:");
        assert!(result.is_err());
    }
}
