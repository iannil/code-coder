//! Utility functions for Zero services.

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
///
/// This function safely handles multi-byte UTF-8 characters (emoji, CJK, accented characters)
/// by using character boundaries instead of byte indices.
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            format!("{}...", truncated.trim_end())
        }
        None => s.to_string(),
    }
}

/// Sanitize a string for safe logging (redact sensitive patterns).
pub fn sanitize_for_log(s: &str) -> String {
    // Patterns that might contain secrets - use double backslash for escaping
    let patterns: &[(&str, &str)] = &[
        (r"(?i)(api[_-]?key|apikey)[=:]\s*\S{20,}", "$1=***REDACTED***"),
        (r"(?i)(password|passwd|pwd)[=:]\s*\S+", "$1=***REDACTED***"),
        (r"(?i)(token|secret|bearer)\s*[=:]\s*\S{10,}", "$1=***REDACTED***"),
        (r"sk-[a-zA-Z0-9]{20,}", "***REDACTED_API_KEY***"),
        (r"ghp_[a-zA-Z0-9]{36}", "***REDACTED_GITHUB_TOKEN***"),
    ];

    let mut result = s.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, *replacement).to_string();
        }
    }
    result
}

/// Parse a duration string like "5m", "1h", "30s" into seconds.
pub fn parse_duration_secs(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".into());
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str.parse().map_err(|_| format!("Invalid number: {num_str}"))?;

    match unit {
        "s" => Ok(num),
        "m" => Ok(num * 60),
        "h" => Ok(num * 3600),
        "d" => Ok(num * 86400),
        _ => Err(format!("Unknown unit: {unit}")),
    }
}

/// Format bytes as human-readable size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(truncate_with_ellipsis("😀😀😀😀", 2), "😀😀...");
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn test_sanitize_for_log() {
        let input = "Using api_key=sk-proj-12345678901234567890";
        let output = sanitize_for_log(input);
        assert!(!output.contains("sk-proj-12345678901234567890"));
        assert!(output.contains("REDACTED"));
    }

    #[test]
    fn test_parse_duration_secs() {
        assert_eq!(parse_duration_secs("30s").unwrap(), 30);
        assert_eq!(parse_duration_secs("5m").unwrap(), 300);
        assert_eq!(parse_duration_secs("1h").unwrap(), 3600);
        assert_eq!(parse_duration_secs("2d").unwrap(), 172800);
        assert!(parse_duration_secs("abc").is_err());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
}
