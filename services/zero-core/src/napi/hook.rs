//! NAPI bindings for Hook pattern matching
//!
//! Provides high-performance pattern matching for hooks using Rust's regex crate
//! with optional SIMD acceleration. This is used for the CPU-intensive parts
//! of hook evaluation while keeping configuration loading and action execution
//! in TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

/// Result of pattern matching
#[napi(object)]
pub struct PatternMatchResult {
    /// Whether any pattern matched
    pub found: bool,
    /// All matched strings
    pub matches: Vec<String>,
    /// Map of pattern to its matches
    pub matches_by_pattern: HashMap<String, Vec<String>>,
}

/// Line match information for content scanning
#[napi(object)]
pub struct ContentMatchResult {
    /// Whether any pattern matched
    pub found: bool,
    /// All matched strings
    pub matches: Vec<String>,
    /// Lines where matches were found (1-indexed)
    pub lines: Vec<u32>,
    /// Map of pattern to line numbers where it matched
    pub lines_by_pattern: HashMap<String, Vec<u32>>,
}

/// Handle to a compiled pattern set for reuse
#[napi]
pub struct PatternSetHandle {
    patterns: Arc<Vec<(String, Regex)>>,
}

/// Create a compiled pattern set for efficient reuse
#[napi]
pub fn create_pattern_set(patterns: Vec<String>) -> Result<PatternSetHandle> {
    let compiled: Result<Vec<_>> = patterns
        .into_iter()
        .map(|p| {
            Regex::new(&p)
                .map(|r| (p.clone(), r))
                .map_err(|e| Error::from_reason(format!("Invalid regex pattern '{}': {}", p, e)))
        })
        .collect();

    Ok(PatternSetHandle {
        patterns: Arc::new(compiled?),
    })
}

#[napi]
impl PatternSetHandle {
    /// Scan content for any matching patterns
    #[napi]
    pub fn scan(&self, content: String) -> PatternMatchResult {
        let mut all_matches = Vec::new();
        let mut matches_by_pattern: HashMap<String, Vec<String>> = HashMap::new();

        for (pattern_str, regex) in self.patterns.iter() {
            let matches: Vec<String> = regex.find_iter(&content).map(|m| m.as_str().to_string()).collect();
            if !matches.is_empty() {
                all_matches.extend(matches.clone());
                matches_by_pattern.insert(pattern_str.clone(), matches);
            }
        }

        PatternMatchResult {
            found: !all_matches.is_empty(),
            matches: all_matches,
            matches_by_pattern,
        }
    }

    /// Scan content and report line numbers for matches
    #[napi]
    pub fn scan_content(&self, content: String) -> ContentMatchResult {
        let lines: Vec<&str> = content.lines().collect();
        let mut all_matches = Vec::new();
        let mut all_lines = Vec::new();
        let mut lines_by_pattern: HashMap<String, Vec<u32>> = HashMap::new();

        for (pattern_str, regex) in self.patterns.iter() {
            let mut pattern_lines = Vec::new();

            for (i, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    let line_num = (i + 1) as u32;
                    if !all_lines.contains(&line_num) {
                        all_lines.push(line_num);
                    }
                    pattern_lines.push(line_num);

                    // Collect the actual matches from this line
                    for m in regex.find_iter(line) {
                        let match_str = m.as_str().to_string();
                        if !all_matches.contains(&match_str) {
                            all_matches.push(match_str);
                        }
                    }
                }
            }

            if !pattern_lines.is_empty() {
                lines_by_pattern.insert(pattern_str.clone(), pattern_lines);
            }
        }

        all_lines.sort();

        ContentMatchResult {
            found: !all_matches.is_empty(),
            matches: all_matches,
            lines: all_lines,
            lines_by_pattern,
        }
    }

    /// Check if a tool name matches a pattern
    #[napi]
    pub fn matches_tool(&self, tool: String) -> bool {
        self.patterns.iter().any(|(_, regex)| regex.is_match(&tool))
    }

    /// Get the number of patterns in this set
    #[napi]
    pub fn pattern_count(&self) -> u32 {
        self.patterns.len() as u32
    }
}

/// Scan content for patterns (one-shot, no precompilation)
#[napi]
pub fn scan_patterns(content: String, patterns: Vec<String>) -> Result<PatternMatchResult> {
    let pattern_set = create_pattern_set(patterns)?;
    Ok(pattern_set.scan(content))
}

/// Scan content for patterns and report line numbers (one-shot)
#[napi]
pub fn scan_content_patterns(content: String, patterns: Vec<String>) -> Result<ContentMatchResult> {
    let pattern_set = create_pattern_set(patterns)?;
    Ok(pattern_set.scan_content(content))
}

/// Check if a string matches a regex pattern (anchored: ^pattern$)
#[napi]
pub fn matches_pattern(pattern: String, value: String) -> Result<bool> {
    let anchored = format!("^{}$", pattern);
    let regex =
        Regex::new(&anchored).map_err(|e| Error::from_reason(format!("Invalid pattern '{}': {}", pattern, e)))?;
    Ok(regex.is_match(&value))
}

/// Check if a string matches a regex pattern (unanchored)
#[napi]
pub fn contains_pattern(pattern: String, value: String) -> Result<bool> {
    let regex =
        Regex::new(&pattern).map_err(|e| Error::from_reason(format!("Invalid pattern '{}': {}", pattern, e)))?;
    Ok(regex.is_match(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_patterns() {
        let result = scan_patterns(
            "API_KEY=secret123 PASSWORD=hunter2".to_string(),
            vec!["API_KEY=\\w+".to_string(), "PASSWORD=\\w+".to_string()],
        )
        .unwrap();

        assert!(result.found);
        assert_eq!(result.matches.len(), 2);
        assert!(result.matches.contains(&"API_KEY=secret123".to_string()));
        assert!(result.matches.contains(&"PASSWORD=hunter2".to_string()));
    }

    #[test]
    fn test_scan_content_patterns() {
        let content = "line 1\nAPI_KEY=secret\nline 3\nPASSWORD=hunter2";
        let result = scan_content_patterns(
            content.to_string(),
            vec!["API_KEY=\\w+".to_string(), "PASSWORD=\\w+".to_string()],
        )
        .unwrap();

        assert!(result.found);
        assert!(result.lines.contains(&2));
        assert!(result.lines.contains(&4));
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("Bash".to_string(), "Bash".to_string()).unwrap());
        assert!(!matches_pattern("Bash".to_string(), "BashCommand".to_string()).unwrap());
        assert!(matches_pattern("Bash.*".to_string(), "BashCommand".to_string()).unwrap());
    }

    #[test]
    fn test_contains_pattern() {
        assert!(contains_pattern("npm".to_string(), "npm install".to_string()).unwrap());
        assert!(contains_pattern("install".to_string(), "npm install".to_string()).unwrap());
        assert!(!contains_pattern("yarn".to_string(), "npm install".to_string()).unwrap());
    }

    #[test]
    fn test_pattern_set_reuse() {
        let set = create_pattern_set(vec!["\\bfoo\\b".to_string(), "\\bbar\\b".to_string()]).unwrap();

        let r1 = set.scan("foo bar baz".to_string());
        assert!(r1.found);
        assert_eq!(r1.matches.len(), 2);

        let r2 = set.scan("qux quux".to_string());
        assert!(!r2.found);

        let r3 = set.scan("foo only".to_string());
        assert!(r3.found);
        assert_eq!(r3.matches.len(), 1);
    }
}
