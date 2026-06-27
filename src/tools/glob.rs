/// ─── GlobTool ──────────────────────────────────────────────────────────────
///
/// Find files matching a glob pattern, respecting .gitignore.
/// Returns matching paths as a JSON array.

use super::Tool;
use globset::Glob;
use ignore::WalkBuilder;

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Input: {\"pattern\":\"**/*.rs\",\"base\":\".\"}. Returns JSON array of paths."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct GlobInput {
            #[serde(default = "default_pattern")]
            pattern: String,
            #[serde(default = "default_base")]
            base: String,
        }

        fn default_pattern() -> String { "**/*".into() }
        fn default_base() -> String { ".".into() }

        let parsed: GlobInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid glob input: {e}. Expected {{\"pattern\":\"...\",\"base\":\"...\"}}"))?;

        let glob = Glob::new(&parsed.pattern)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern '{}': {e}", parsed.pattern))?;
        let glob_matcher = glob.compile_matcher();

        let walker = WalkBuilder::new(&parsed.base)
            .standard_filters(true)   // respect .gitignore, hidden files, etc.
            .build();

        let mut results: Vec<String> = Vec::new();
        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|t| t.is_file()) {
                        let path = entry.path();
                        if glob_matcher.is_match(path) {
                            if let Ok(rel) = path.strip_prefix(&parsed.base) {
                                results.push(rel.to_string_lossy().to_string());
                            } else {
                                results.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        results.sort();
        Ok(serde_json::to_string(&results)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_finds_rust_files() {
        let tool = GlobTool;
        let input = r#"{"pattern": "**/*.rs", "base": "src"}"#;
        let result = tool.execute(input).unwrap();
        let paths: Vec<String> = serde_json::from_str(&result).unwrap();
        assert!(paths.contains(&"main.rs".to_string()), "should find main.rs, got: {paths:?}");
        assert!(paths.contains(&"tools/mod.rs".to_string()), "should find tools/mod.rs");
    }

    #[test]
    fn test_glob_invalid_pattern() {
        let tool = GlobTool;
        let result = tool.execute(r#"{"pattern": "[invalid", "base": "."}"#);
        assert!(result.is_err(), "invalid pattern should fail");
    }

    #[test]
    fn test_glob_empty_input_defaults() {
        let tool = GlobTool;
        let result = tool.execute(r#"{}"#);
        assert!(result.is_ok(), "empty input should use defaults: {:?}", result.err());
    }

    #[test]
    fn test_glob_no_matches() {
        let tool = GlobTool;
        let result = tool.execute(r#"{"pattern": "**/*.nonexistent", "base": "."}"#).unwrap();
        let paths: Vec<String> = serde_json::from_str(&result).unwrap();
        assert!(paths.is_empty());
    }
}
