//! NAPI bindings for Hook pattern matching
//!
//! Provides high-performance pattern matching for hooks using Rust's regex crate
//! with optional SIMD acceleration. This is used for the CPU-intensive parts
//! of hook evaluation while keeping configuration loading and action execution
//! in TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use regex::Regex;
use serde::{Deserialize, Serialize};
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

// ============================================================================
// Phase 14: Extended Hook Configuration Handle
// ============================================================================

/// Action type enum (matching TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Scan,
    CheckEnv,
    CheckStyle,
    NotifyOnly,
    ScanContent,
    RunCommand,
    AnalyzeChanges,
    ScanFiles,
}

/// Hook action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    #[serde(rename = "type")]
    pub action_type: ActionType,
    #[serde(default)]
    pub patterns: Vec<String>,
    pub message: Option<String>,
    #[serde(default)]
    pub block: bool,
    pub command: Option<String>,
    #[serde(default)]
    pub r#async: bool,
    pub variable: Option<String>,
    pub command_pattern: Option<String>,
    pub file_pattern: Option<String>,
    #[serde(default)]
    pub on_output: HashMap<String, String>,
    pub on_vulnerabilities: Option<String>,
}

/// Hook definition with compiled patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub pattern: Option<String>,
    pub description: Option<String>,
    pub command_pattern: Option<String>,
    pub file_pattern: Option<String>,
    pub actions: Vec<HookAction>,
}

/// Lifecycle type
#[napi(string_enum)]
pub enum NapiHookLifecycle {
    PreToolUse,
    PostToolUse,
    PreResponse,
    Stop,
}

/// Hook evaluation context
#[napi(object)]
pub struct NapiHookContext {
    pub tool: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub file_content: Option<String>,
    pub input_json: Option<String>,
    pub output: Option<String>,
}

/// Hook match result
#[napi(object)]
pub struct HookMatchResult {
    /// Name of the matching hook
    pub hook_name: String,
    /// Indices of matching actions (for efficient action selection)
    pub matching_action_indices: Vec<u32>,
    /// Whether any action has block=true
    pub has_blocking_action: bool,
}

/// Hook evaluation result
#[napi(object)]
pub struct HookEvalResult {
    /// Hooks that matched the context
    pub matching_hooks: Vec<HookMatchResult>,
    /// Total number of hooks evaluated
    pub total_hooks_evaluated: u32,
    /// Evaluation duration in microseconds
    pub eval_duration_us: u32,
}

/// Compiled hook configuration for efficient evaluation
struct CompiledHook {
    name: String,
    definition: HookDefinition,
    pattern: Option<Regex>,
    command_pattern: Option<Regex>,
    file_pattern: Option<Regex>,
    action_patterns: Vec<CompiledAction>,
}

struct CompiledAction {
    action: HookAction,
    patterns: Option<PatternSetHandle>,
    command_pattern: Option<Regex>,
    file_pattern: Option<Regex>,
}

/// Handle to compiled hook configuration
#[napi]
pub struct HookConfigHandle {
    hooks_by_lifecycle: HashMap<String, Vec<CompiledHook>>,
}

#[napi]
impl HookConfigHandle {
    /// Create a new hook config handle from JSON configuration
    #[napi(constructor)]
    pub fn new(config_json: String) -> Result<Self> {
        let config: HooksConfig = serde_json::from_str(&config_json)
            .map_err(|e| Error::from_reason(format!("Failed to parse hook config: {}", e)))?;

        let mut hooks_by_lifecycle: HashMap<String, Vec<CompiledHook>> = HashMap::new();

        for (lifecycle, hooks_map) in [
            ("PreToolUse", &config.hooks.pre_tool_use),
            ("PostToolUse", &config.hooks.post_tool_use),
            ("PreResponse", &config.hooks.pre_response),
            ("Stop", &config.hooks.stop),
        ] {
            if let Some(hooks) = hooks_map {
                let compiled: Result<Vec<CompiledHook>> = hooks
                    .iter()
                    .map(|(name, def)| compile_hook(name.clone(), def.clone()))
                    .collect();
                hooks_by_lifecycle.insert(lifecycle.to_string(), compiled?);
            }
        }

        Ok(Self { hooks_by_lifecycle })
    }

    /// Evaluate hooks for a given lifecycle and context
    #[napi]
    pub fn evaluate(&self, lifecycle: NapiHookLifecycle, ctx: NapiHookContext) -> HookEvalResult {
        let start = std::time::Instant::now();
        let lifecycle_str = match lifecycle {
            NapiHookLifecycle::PreToolUse => "PreToolUse",
            NapiHookLifecycle::PostToolUse => "PostToolUse",
            NapiHookLifecycle::PreResponse => "PreResponse",
            NapiHookLifecycle::Stop => "Stop",
        };

        let hooks = match self.hooks_by_lifecycle.get(lifecycle_str) {
            Some(h) => h,
            None => return HookEvalResult {
                matching_hooks: vec![],
                total_hooks_evaluated: 0,
                eval_duration_us: start.elapsed().as_micros() as u32,
            },
        };

        let mut matching_hooks = Vec::new();
        let mut total_evaluated = 0u32;

        for hook in hooks {
            total_evaluated += 1;

            // Check hook-level patterns
            if let Some(ref pattern) = hook.pattern {
                if let Some(ref tool) = ctx.tool {
                    if !pattern.is_match(tool) {
                        continue;
                    }
                }
            }

            if let Some(ref pattern) = hook.command_pattern {
                if let Some(ref cmd) = ctx.command {
                    if !pattern.is_match(cmd) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(ref pattern) = hook.file_pattern {
                if let Some(ref path) = ctx.file_path {
                    if !pattern.is_match(path) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Check action-level patterns
            let mut matching_indices = Vec::new();
            let mut has_blocking = false;

            for (idx, compiled_action) in hook.action_patterns.iter().enumerate() {
                let action_matches = evaluate_action_patterns(compiled_action, &ctx);
                if action_matches {
                    matching_indices.push(idx as u32);
                    if compiled_action.action.block {
                        has_blocking = true;
                    }
                }
            }

            if !matching_indices.is_empty() {
                matching_hooks.push(HookMatchResult {
                    hook_name: hook.name.clone(),
                    matching_action_indices: matching_indices,
                    has_blocking_action: has_blocking,
                });
            }
        }

        HookEvalResult {
            matching_hooks,
            total_hooks_evaluated: total_evaluated,
            eval_duration_us: start.elapsed().as_micros() as u32,
        }
    }

    /// Get the number of hooks for a lifecycle
    #[napi]
    pub fn hook_count(&self, lifecycle: NapiHookLifecycle) -> u32 {
        let lifecycle_str = match lifecycle {
            NapiHookLifecycle::PreToolUse => "PreToolUse",
            NapiHookLifecycle::PostToolUse => "PostToolUse",
            NapiHookLifecycle::PreResponse => "PreResponse",
            NapiHookLifecycle::Stop => "Stop",
        };
        self.hooks_by_lifecycle.get(lifecycle_str).map(|h| h.len() as u32).unwrap_or(0)
    }

    /// Scan content for patterns using a specific action's patterns
    #[napi]
    pub fn scan_action_patterns(&self, lifecycle: NapiHookLifecycle, hook_name: String, action_index: u32, content: String) -> PatternMatchResult {
        let lifecycle_str = match lifecycle {
            NapiHookLifecycle::PreToolUse => "PreToolUse",
            NapiHookLifecycle::PostToolUse => "PostToolUse",
            NapiHookLifecycle::PreResponse => "PreResponse",
            NapiHookLifecycle::Stop => "Stop",
        };

        let hooks = match self.hooks_by_lifecycle.get(lifecycle_str) {
            Some(h) => h,
            None => return PatternMatchResult { found: false, matches: vec![], matches_by_pattern: HashMap::new() },
        };

        for hook in hooks {
            if hook.name == hook_name {
                if let Some(action) = hook.action_patterns.get(action_index as usize) {
                    if let Some(ref patterns) = action.patterns {
                        return patterns.scan(content);
                    }
                }
            }
        }

        PatternMatchResult { found: false, matches: vec![], matches_by_pattern: HashMap::new() }
    }
}

// Hook config JSON structure
#[derive(Debug, Deserialize)]
struct HooksConfig {
    hooks: HooksMap,
    #[serde(default)]
    settings: Option<HookSettings>,
}

#[derive(Debug, Deserialize)]
struct HooksMap {
    #[serde(rename = "PreToolUse")]
    pre_tool_use: Option<HashMap<String, HookDefinition>>,
    #[serde(rename = "PostToolUse")]
    post_tool_use: Option<HashMap<String, HookDefinition>>,
    #[serde(rename = "PreResponse")]
    pre_response: Option<HashMap<String, HookDefinition>>,
    #[serde(rename = "Stop")]
    stop: Option<HashMap<String, HookDefinition>>,
}

#[derive(Debug, Deserialize)]
struct HookSettings {
    #[serde(default = "default_true")]
    enabled: bool,
    blocking_mode: Option<String>,
    log_level: Option<String>,
}

fn default_true() -> bool { true }

fn compile_hook(name: String, def: HookDefinition) -> Result<CompiledHook> {
    let pattern = def.pattern.as_ref()
        .map(|p| {
            let anchored = format!("^{}$", p);
            Regex::new(&anchored).map_err(|e| Error::from_reason(format!("Invalid hook pattern '{}': {}", p, e)))
        })
        .transpose()?;

    let command_pattern = def.command_pattern.as_ref()
        .map(|p| Regex::new(p).map_err(|e| Error::from_reason(format!("Invalid command pattern '{}': {}", p, e))))
        .transpose()?;

    let file_pattern = def.file_pattern.as_ref()
        .map(|p| Regex::new(p).map_err(|e| Error::from_reason(format!("Invalid file pattern '{}': {}", p, e))))
        .transpose()?;

    let action_patterns: Result<Vec<CompiledAction>> = def.actions.iter()
        .map(|action| compile_action(action.clone()))
        .collect();

    Ok(CompiledHook {
        name,
        definition: def,
        pattern,
        command_pattern,
        file_pattern,
        action_patterns: action_patterns?,
    })
}

fn compile_action(action: HookAction) -> Result<CompiledAction> {
    let patterns = if !action.patterns.is_empty() {
        Some(create_pattern_set(action.patterns.clone())?)
    } else {
        None
    };

    let command_pattern = action.command_pattern.as_ref()
        .map(|p| Regex::new(p).map_err(|e| Error::from_reason(format!("Invalid action command pattern '{}': {}", p, e))))
        .transpose()?;

    let file_pattern = action.file_pattern.as_ref()
        .map(|p| Regex::new(p).map_err(|e| Error::from_reason(format!("Invalid action file pattern '{}': {}", p, e))))
        .transpose()?;

    Ok(CompiledAction {
        action,
        patterns,
        command_pattern,
        file_pattern,
    })
}

fn evaluate_action_patterns(action: &CompiledAction, ctx: &NapiHookContext) -> bool {
    // Check action-level command pattern
    if let Some(ref pattern) = action.command_pattern {
        match &ctx.command {
            Some(cmd) => if !pattern.is_match(cmd) { return false; },
            None => return false,
        }
    }

    // Check action-level file pattern
    if let Some(ref pattern) = action.file_pattern {
        match &ctx.file_path {
            Some(path) => if !pattern.is_match(path) { return false; },
            None => return false,
        }
    }

    true
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

    #[test]
    fn test_hook_config_handle() {
        let config = r#"{
            "hooks": {
                "PreToolUse": {
                    "sensitive-scan": {
                        "pattern": "Bash",
                        "actions": [
                            {
                                "type": "scan",
                                "patterns": ["API_KEY", "SECRET"],
                                "block": true,
                                "message": "Sensitive pattern detected"
                            }
                        ]
                    }
                }
            }
        }"#;

        let handle = HookConfigHandle::new(config.to_string()).unwrap();
        assert_eq!(handle.hook_count(NapiHookLifecycle::PreToolUse), 1);
        assert_eq!(handle.hook_count(NapiHookLifecycle::PostToolUse), 0);

        let ctx = NapiHookContext {
            tool: Some("Bash".to_string()),
            command: Some("echo API_KEY=secret".to_string()),
            file_path: None,
            file_content: None,
            input_json: None,
            output: None,
        };

        let result = handle.evaluate(NapiHookLifecycle::PreToolUse, ctx);
        assert_eq!(result.matching_hooks.len(), 1);
        assert!(result.matching_hooks[0].has_blocking_action);
    }
}
