//! Risk evaluation for tool calls.
//!
//! Evaluates the risk level of tool operations to determine whether they
//! should be auto-approved, queued for human review, or denied.
//!
//! Risk levels:
//! - Safe: Read-only operations with no side effects (Read, Glob)
//! - Low: Read-only with potential network requests (Grep, WebSearch, WebFetch)
//! - Medium: Limited write operations (Edit small files, Write docs)
//! - High: Destructive write operations (Bash, Edit code, Write)
//! - Critical: Irreversible operations (git push, rm -rf, payments)

use serde::{Deserialize, Serialize};

/// Risk level for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Read-only, no side effects
    Safe = 0,
    /// Read-only, may have network requests
    Low = 1,
    /// Limited write operations
    Medium = 2,
    /// Destructive write operations
    High = 3,
    /// Irreversible operations
    Critical = 4,
}

impl RiskLevel {
    /// Get the numeric value for comparison.
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Get the display name in Chinese.
    pub fn display_name(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "安全",
            RiskLevel::Low => "低风险",
            RiskLevel::Medium => "中风险",
            RiskLevel::High => "高风险",
            RiskLevel::Critical => "极高风险",
        }
    }

    /// Get the icon for this risk level.
    pub fn icon(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "🟢",
            RiskLevel::Low => "🟡",
            RiskLevel::Medium => "🟠",
            RiskLevel::High => "🔴",
            RiskLevel::Critical => "💀",
        }
    }
}

impl Default for RiskLevel {
    fn default() -> Self {
        RiskLevel::Medium
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Tool classification for risk evaluation.
#[derive(Debug, Clone)]
pub struct ToolClassification {
    /// Tool name
    pub name: String,
    /// Base risk level
    pub base_risk: RiskLevel,
    /// Patterns that elevate risk
    pub risk_patterns: Vec<RiskPattern>,
}

/// Pattern that can modify risk level.
#[derive(Debug, Clone)]
pub struct RiskPattern {
    /// Pattern to match in arguments (regex or substring)
    pub pattern: String,
    /// Whether this is a regex pattern
    pub is_regex: bool,
    /// Risk level adjustment (positive = more risky)
    pub adjustment: i8,
    /// Description of why this pattern is risky
    pub reason: String,
}

/// Risk evaluator for tool calls.
#[derive(Debug, Clone)]
pub struct RiskEvaluator {
    /// Tool classifications
    classifications: Vec<ToolClassification>,
    /// Default risk level for unknown tools
    default_risk: RiskLevel,
}

impl Default for RiskEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskEvaluator {
    /// Create a new risk evaluator with default classifications.
    pub fn new() -> Self {
        Self {
            classifications: Self::default_classifications(),
            default_risk: RiskLevel::High, // Unknown tools are high risk
        }
    }

    /// Evaluate the risk level of a tool call.
    pub fn evaluate(&self, tool: &str, args: &serde_json::Value) -> RiskEvaluation {
        // Find classification for this tool
        let classification = self.classifications
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(tool));

        let (base_risk, patterns) = match classification {
            Some(c) => (c.base_risk, &c.risk_patterns),
            None => {
                return RiskEvaluation {
                    tool: tool.to_string(),
                    risk_level: self.default_risk,
                    reasons: vec!["Unknown tool - applying default high risk".to_string()],
                    adjustments: vec![],
                };
            }
        };

        // Check for risk patterns in arguments
        let args_str = serde_json::to_string(args).unwrap_or_default();
        let mut adjustments = Vec::new();
        let mut total_adjustment: i8 = 0;

        for pattern in patterns {
            let matches = if pattern.is_regex {
                regex::Regex::new(&pattern.pattern)
                    .map(|r| r.is_match(&args_str))
                    .unwrap_or(false)
            } else {
                args_str.contains(&pattern.pattern)
            };

            if matches {
                adjustments.push(RiskAdjustment {
                    pattern: pattern.pattern.clone(),
                    adjustment: pattern.adjustment,
                    reason: pattern.reason.clone(),
                });
                total_adjustment = total_adjustment.saturating_add(pattern.adjustment);
            }
        }

        // Calculate final risk level
        let base_value = base_risk.value() as i8;
        let final_value = (base_value + total_adjustment).clamp(0, 4) as u8;
        let risk_level = match final_value {
            0 => RiskLevel::Safe,
            1 => RiskLevel::Low,
            2 => RiskLevel::Medium,
            3 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        let mut reasons = vec![format!("Base risk for {}: {}", tool, base_risk.display_name())];
        for adj in &adjustments {
            reasons.push(format!("{}: {}", adj.reason, if adj.adjustment > 0 { "+" } else { "" }.to_string() + &adj.adjustment.to_string()));
        }

        RiskEvaluation {
            tool: tool.to_string(),
            risk_level,
            reasons,
            adjustments,
        }
    }

    /// Check if a tool is in the safe tools set.
    pub fn is_safe_tool(&self, tool: &str) -> bool {
        self.classifications
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(tool))
            .map_or(false, |c| c.base_risk == RiskLevel::Safe)
    }

    /// Get all tools at or below a risk level.
    pub fn tools_at_or_below(&self, max_risk: RiskLevel) -> Vec<&str> {
        self.classifications
            .iter()
            .filter(|c| c.base_risk <= max_risk)
            .map(|c| c.name.as_str())
            .collect()
    }

    /// Default tool classifications based on typical Claude Code tools.
    fn default_classifications() -> Vec<ToolClassification> {
        vec![
            // Safe: Read-only, no side effects
            ToolClassification {
                name: "Read".to_string(),
                base_risk: RiskLevel::Safe,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "Glob".to_string(),
                base_risk: RiskLevel::Safe,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "LS".to_string(),
                base_risk: RiskLevel::Safe,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "NotebookRead".to_string(),
                base_risk: RiskLevel::Safe,
                risk_patterns: vec![],
            },

            // Low: Read-only with network/search
            ToolClassification {
                name: "Grep".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "WebSearch".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "WebFetch".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![
                    RiskPattern {
                        pattern: "localhost".to_string(),
                        is_regex: false,
                        adjustment: 1,
                        reason: "Accessing localhost may expose internal services".to_string(),
                    },
                    RiskPattern {
                        pattern: "127\\.0\\.0\\.1".to_string(),
                        is_regex: true,
                        adjustment: 1,
                        reason: "Accessing localhost may expose internal services".to_string(),
                    },
                ],
            },
            ToolClassification {
                name: "Task".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![],
            },

            // Medium: Limited writes
            ToolClassification {
                name: "Edit".to_string(),
                base_risk: RiskLevel::Medium,
                risk_patterns: vec![
                    RiskPattern {
                        pattern: "\\.env".to_string(),
                        is_regex: true,
                        adjustment: 2,
                        reason: "Editing .env files may expose secrets".to_string(),
                    },
                    RiskPattern {
                        pattern: "credentials".to_string(),
                        is_regex: false,
                        adjustment: 2,
                        reason: "Editing credential files is sensitive".to_string(),
                    },
                    RiskPattern {
                        pattern: "password".to_string(),
                        is_regex: false,
                        adjustment: 1,
                        reason: "Editing password-related files".to_string(),
                    },
                ],
            },
            ToolClassification {
                name: "NotebookEdit".to_string(),
                base_risk: RiskLevel::Medium,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "TodoWrite".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![],
            },

            // High: Destructive writes
            ToolClassification {
                name: "Write".to_string(),
                base_risk: RiskLevel::High,
                risk_patterns: vec![
                    RiskPattern {
                        pattern: "\\.env".to_string(),
                        is_regex: true,
                        adjustment: 1,
                        reason: "Writing .env files may expose secrets".to_string(),
                    },
                    RiskPattern {
                        pattern: "\\.(md|txt|rst)$".to_string(),
                        is_regex: true,
                        adjustment: -1, // Documentation is lower risk
                        reason: "Documentation files are lower risk".to_string(),
                    },
                ],
            },
            ToolClassification {
                name: "Bash".to_string(),
                base_risk: RiskLevel::High,
                risk_patterns: vec![
                    // Critical patterns
                    RiskPattern {
                        pattern: "rm\\s+-rf".to_string(),
                        is_regex: true,
                        adjustment: 2,
                        reason: "Recursive force delete is dangerous".to_string(),
                    },
                    RiskPattern {
                        pattern: "git\\s+push".to_string(),
                        is_regex: true,
                        adjustment: 1,
                        reason: "Git push modifies remote repository".to_string(),
                    },
                    RiskPattern {
                        pattern: "git\\s+push\\s+--force".to_string(),
                        is_regex: true,
                        adjustment: 2,
                        reason: "Force push can destroy remote history".to_string(),
                    },
                    RiskPattern {
                        pattern: "git\\s+reset\\s+--hard".to_string(),
                        is_regex: true,
                        adjustment: 1,
                        reason: "Hard reset can lose uncommitted changes".to_string(),
                    },
                    RiskPattern {
                        pattern: "sudo".to_string(),
                        is_regex: false,
                        adjustment: 2,
                        reason: "Sudo grants elevated privileges".to_string(),
                    },
                    RiskPattern {
                        pattern: "chmod\\s+777".to_string(),
                        is_regex: true,
                        adjustment: 1,
                        reason: "chmod 777 is insecure".to_string(),
                    },
                    // Lower risk patterns
                    RiskPattern {
                        pattern: "git\\s+status".to_string(),
                        is_regex: true,
                        adjustment: -2,
                        reason: "Git status is read-only".to_string(),
                    },
                    RiskPattern {
                        pattern: "git\\s+log".to_string(),
                        is_regex: true,
                        adjustment: -2,
                        reason: "Git log is read-only".to_string(),
                    },
                    RiskPattern {
                        pattern: "git\\s+diff".to_string(),
                        is_regex: true,
                        adjustment: -2,
                        reason: "Git diff is read-only".to_string(),
                    },
                    RiskPattern {
                        pattern: "ls\\b".to_string(),
                        is_regex: true,
                        adjustment: -2,
                        reason: "ls is read-only".to_string(),
                    },
                    RiskPattern {
                        pattern: "pwd\\b".to_string(),
                        is_regex: true,
                        adjustment: -2,
                        reason: "pwd is read-only".to_string(),
                    },
                    RiskPattern {
                        pattern: "echo\\b".to_string(),
                        is_regex: true,
                        adjustment: -1,
                        reason: "echo is generally safe".to_string(),
                    },
                ],
            },

            // Playwright/Browser tools
            ToolClassification {
                name: "mcp__playwright__browser_navigate".to_string(),
                base_risk: RiskLevel::Medium,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "mcp__playwright__browser_click".to_string(),
                base_risk: RiskLevel::Medium,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "mcp__playwright__browser_snapshot".to_string(),
                base_risk: RiskLevel::Low,
                risk_patterns: vec![],
            },
            ToolClassification {
                name: "mcp__playwright__browser_type".to_string(),
                base_risk: RiskLevel::Medium,
                risk_patterns: vec![
                    RiskPattern {
                        pattern: "password".to_string(),
                        is_regex: false,
                        adjustment: 1,
                        reason: "Typing passwords requires caution".to_string(),
                    },
                ],
            },
        ]
    }
}

/// Result of risk evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskEvaluation {
    /// Tool name
    pub tool: String,
    /// Final risk level
    pub risk_level: RiskLevel,
    /// Reasons for the risk level
    pub reasons: Vec<String>,
    /// Risk adjustments applied
    pub adjustments: Vec<RiskAdjustment>,
}

/// A risk adjustment from pattern matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAdjustment {
    /// Pattern that matched
    pub pattern: String,
    /// Risk adjustment value
    pub adjustment: i8,
    /// Reason for adjustment
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_safe_tools() {
        let evaluator = RiskEvaluator::new();

        let eval = evaluator.evaluate("Read", &json!({"file_path": "/test.txt"}));
        assert_eq!(eval.risk_level, RiskLevel::Safe);

        let eval = evaluator.evaluate("Glob", &json!({"pattern": "*.rs"}));
        assert_eq!(eval.risk_level, RiskLevel::Safe);
    }

    #[test]
    fn test_low_risk_tools() {
        let evaluator = RiskEvaluator::new();

        let eval = evaluator.evaluate("Grep", &json!({"pattern": "TODO"}));
        assert_eq!(eval.risk_level, RiskLevel::Low);

        let eval = evaluator.evaluate("WebSearch", &json!({"query": "rust programming"}));
        assert_eq!(eval.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_bash_risk_patterns() {
        let evaluator = RiskEvaluator::new();

        // High risk bash command
        let eval = evaluator.evaluate("Bash", &json!({"command": "rm -rf /tmp/test"}));
        assert_eq!(eval.risk_level, RiskLevel::Critical);

        // Lower risk bash command (read-only)
        let eval = evaluator.evaluate("Bash", &json!({"command": "git status"}));
        assert!(eval.risk_level <= RiskLevel::Medium);

        // Sudo elevates risk
        let eval = evaluator.evaluate("Bash", &json!({"command": "sudo apt install foo"}));
        assert_eq!(eval.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_edit_sensitive_files() {
        let evaluator = RiskEvaluator::new();

        // Normal edit
        let eval = evaluator.evaluate("Edit", &json!({"file_path": "/src/main.rs"}));
        assert_eq!(eval.risk_level, RiskLevel::Medium);

        // .env file elevates risk
        let eval = evaluator.evaluate("Edit", &json!({"file_path": "/.env"}));
        assert_eq!(eval.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_unknown_tool() {
        let evaluator = RiskEvaluator::new();

        let eval = evaluator.evaluate("UnknownTool", &json!({}));
        assert_eq!(eval.risk_level, RiskLevel::High);
        assert!(eval.reasons[0].contains("Unknown tool"));
    }

    #[test]
    fn test_tools_at_or_below() {
        let evaluator = RiskEvaluator::new();

        let safe_tools = evaluator.tools_at_or_below(RiskLevel::Safe);
        assert!(safe_tools.contains(&"Read"));
        assert!(safe_tools.contains(&"Glob"));
        assert!(!safe_tools.contains(&"Bash"));

        let low_tools = evaluator.tools_at_or_below(RiskLevel::Low);
        assert!(low_tools.contains(&"Read"));
        assert!(low_tools.contains(&"Grep"));
        assert!(low_tools.contains(&"WebSearch"));
    }
}
