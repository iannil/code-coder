//! Risk Assessment for Tool Operations
//!
//! This module provides risk assessment for Bash commands and file operations,
//! enabling auto-approval decisions in autonomous/unattended mode.
//!
//! # Design
//!
//! Risk levels are ordered from least to most dangerous:
//! - **Safe**: Read-only operations with no side effects
//! - **Low**: Read operations with minimal risk
//! - **Medium**: Local reversible writes
//! - **High**: External effects or semi-reversible operations
//! - **Critical**: Destructive or system-level operations
//!
//! # Performance
//!
//! All regex patterns are pre-compiled using `OnceLock` for zero-cost
//! repeated evaluations. First call initializes patterns (~1ms),
//! subsequent calls are pure matching (~1μs).

use regex::Regex;
use std::sync::OnceLock;

// ============================================================================
// Types
// ============================================================================

/// Risk levels for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RiskLevel {
    /// No side effects, read-only
    Safe = 0,
    /// Minimal risk, external read-only
    Low = 1,
    /// Local reversible writes
    Medium = 2,
    /// External effects or semi-reversible
    High = 3,
    /// Destructive or system-level
    Critical = 4,
}

impl RiskLevel {
    /// Get numeric value for comparison
    pub fn value(self) -> u8 {
        self as u8
    }

    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "safe" => Some(RiskLevel::Safe),
            "low" => Some(RiskLevel::Low),
            "medium" => Some(RiskLevel::Medium),
            "high" => Some(RiskLevel::High),
            "critical" => Some(RiskLevel::Critical),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "safe",
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Result of a risk assessment
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    /// Assessed risk level
    pub risk: RiskLevel,
    /// Human-readable reason for the assessment
    pub reason: &'static str,
}

impl RiskAssessment {
    /// Create a new risk assessment
    pub fn new(risk: RiskLevel, reason: &'static str) -> Self {
        Self { risk, reason }
    }

    /// Check if this risk can be auto-approved (not critical)
    pub fn auto_approvable(&self) -> bool {
        self.risk != RiskLevel::Critical
    }
}

// ============================================================================
// Bash Risk Patterns
// ============================================================================

/// A compiled risk pattern for Bash commands
struct BashRiskPattern {
    regex: Regex,
    risk: RiskLevel,
    reason: &'static str,
}

/// Pre-compiled Bash risk patterns
fn bash_patterns() -> &'static [BashRiskPattern] {
    static PATTERNS: OnceLock<Vec<BashRiskPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // ================================================================
            // CRITICAL - System level, destructive, irreversible
            // ================================================================
            BashRiskPattern {
                regex: Regex::new(r"\bsudo\b").unwrap(),
                risk: RiskLevel::Critical,
                reason: "sudo command requires elevated privileges",
            },
            BashRiskPattern {
                regex: Regex::new(r"\brm\s+(-[rRf]+\s+)*\/\s*$").unwrap(),
                risk: RiskLevel::Critical,
                reason: "rm on root path is destructive",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(shutdown|reboot|init|poweroff|halt)\b").unwrap(),
                risk: RiskLevel::Critical,
                reason: "system control command",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(mkfs|fdisk|parted|dd)\b").unwrap(),
                risk: RiskLevel::Critical,
                reason: "disk manipulation command",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(chmod|chown)\s+(-R\s+)?[0-9]{3,4}\s+\/").unwrap(),
                risk: RiskLevel::Critical,
                reason: "permission change on root",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bgit\s+push\s+(-f|--force)").unwrap(),
                risk: RiskLevel::Critical,
                reason: "force push is destructive",
            },
            BashRiskPattern {
                regex: Regex::new(r"\brm\s+-[rRf]*\s+\*").unwrap(),
                risk: RiskLevel::Critical,
                reason: "recursive delete with wildcard",
            },
            BashRiskPattern {
                regex: Regex::new(r">\s*/dev/(sd[a-z]|nvme|hd[a-z])").unwrap(),
                risk: RiskLevel::Critical,
                reason: "writing to raw disk device",
            },

            // ================================================================
            // HIGH - External effects, semi-reversible
            // ================================================================
            BashRiskPattern {
                regex: Regex::new(r"\brm\s+-[rRf]+").unwrap(),
                risk: RiskLevel::High,
                reason: "recursive file deletion",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bgit\s+push\b").unwrap(),
                risk: RiskLevel::High,
                reason: "git push has external effects",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bgit\s+reset\s+--hard").unwrap(),
                risk: RiskLevel::High,
                reason: "hard reset discards changes",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcurl\s+.*-X\s*(POST|PUT|DELETE|PATCH)").unwrap(),
                risk: RiskLevel::High,
                reason: "HTTP mutation request",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcurl\s+.*(-d|--data)").unwrap(),
                risk: RiskLevel::High,
                reason: "HTTP request with data payload",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bnpm\s+publish\b").unwrap(),
                risk: RiskLevel::High,
                reason: "package publishing has external effects",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcargo\s+publish\b").unwrap(),
                risk: RiskLevel::High,
                reason: "package publishing has external effects",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bdocker\s+(push|rm|rmi)\b").unwrap(),
                risk: RiskLevel::High,
                reason: "docker registry/image manipulation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bkubectl\s+(delete|apply|patch)\b").unwrap(),
                risk: RiskLevel::High,
                reason: "kubernetes resource modification",
            },
            BashRiskPattern {
                regex: Regex::new(r"\baws\s+.*(delete|remove|terminate)").unwrap(),
                risk: RiskLevel::High,
                reason: "AWS resource deletion",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bgcloud\s+.*(delete|remove)").unwrap(),
                risk: RiskLevel::High,
                reason: "GCP resource deletion",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bscp\b").unwrap(),
                risk: RiskLevel::High,
                reason: "remote file copy",
            },
            BashRiskPattern {
                regex: Regex::new(r"\brsync\b").unwrap(),
                risk: RiskLevel::High,
                reason: "remote file sync",
            },

            // ================================================================
            // MEDIUM - Local changes, reversible
            // ================================================================
            BashRiskPattern {
                regex: Regex::new(r"\bgit\s+(add|commit|checkout|branch|merge|rebase)\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "git local operation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(npm|pnpm|yarn|bun)\s+(install|uninstall|add|remove)\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "dependency modification",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcargo\s+(add|remove|update)\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "dependency modification",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bmkdir\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "directory creation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\btouch\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "file creation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bmv\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "file move/rename",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcp\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "file copy",
            },
            BashRiskPattern {
                regex: Regex::new(r"\brm\s+[^-]").unwrap(),
                risk: RiskLevel::Medium,
                reason: "file deletion",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bdocker\s+(build|run|exec)\b").unwrap(),
                risk: RiskLevel::Medium,
                reason: "docker container operation",
            },

            // ================================================================
            // LOW - Information gathering, read-only
            // ================================================================
            BashRiskPattern {
                regex: Regex::new(r"\bgit\s+(status|log|diff|show|branch\s+-[avl]|remote\s+-v)\b").unwrap(),
                risk: RiskLevel::Low,
                reason: "git read operation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bcurl\s+.*-X\s*GET").unwrap(),
                risk: RiskLevel::Low,
                reason: "HTTP read request",
            },
            // Note: Simple curl without mutation flags is handled by the default "unknown" case
            // since Rust regex doesn't support negative lookahead
            BashRiskPattern {
                regex: Regex::new(r"\b(ls|cat|head|tail|less|more|pwd|which|whoami|echo|env|printenv)\b").unwrap(),
                risk: RiskLevel::Low,
                reason: "read/info operation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(grep|rg|find|fd|tree|wc|sort|uniq|diff)\b").unwrap(),
                risk: RiskLevel::Low,
                reason: "search/analysis operation",
            },
            BashRiskPattern {
                regex: Regex::new(r"\b(npm|pnpm|yarn|bun|cargo)\s+(list|outdated|info|show|search|view)\b").unwrap(),
                risk: RiskLevel::Low,
                reason: "package info query",
            },
            BashRiskPattern {
                regex: Regex::new(r"\bdocker\s+(ps|images|logs|inspect)\b").unwrap(),
                risk: RiskLevel::Low,
                reason: "docker info query",
            },
        ]
    })
}

// ============================================================================
// File Risk Patterns
// ============================================================================

/// Assess risk for Bash command
///
/// # Arguments
/// * `command` - The Bash command to assess
///
/// # Returns
/// A `RiskAssessment` with the highest matching risk level
///
/// # Example
/// ```
/// use zero_core::security::risk::assess_bash_risk;
///
/// let result = assess_bash_risk("git push origin main");
/// assert_eq!(result.risk.as_str(), "high");
///
/// let result = assess_bash_risk("ls -la");
/// assert_eq!(result.risk.as_str(), "low");
/// ```
pub fn assess_bash_risk(command: &str) -> RiskAssessment {
    let mut max_risk = RiskLevel::Safe;
    let mut max_reason = "No risky patterns detected";

    for pattern in bash_patterns() {
        if pattern.regex.is_match(command) && pattern.risk > max_risk {
            max_risk = pattern.risk;
            max_reason = pattern.reason;
        }
    }

    // Special case: curl without mutation flags should be low risk
    // Rust regex doesn't support negative lookahead, so we handle this separately
    if max_risk == RiskLevel::Safe && command.contains("curl") {
        // Check if it's a simple curl (no POST/PUT/DELETE/PATCH, no data)
        let is_mutation = command.contains("-X POST")
            || command.contains("-X PUT")
            || command.contains("-X DELETE")
            || command.contains("-X PATCH")
            || command.contains("--data")
            || command.contains(" -d ");

        if !is_mutation {
            return RiskAssessment::new(RiskLevel::Low, "HTTP GET request (default)");
        }
    }

    // If no pattern matched, Bash commands default to high (could be anything)
    if max_risk == RiskLevel::Safe {
        RiskAssessment::new(RiskLevel::High, "Unknown bash command")
    } else {
        RiskAssessment::new(max_risk, max_reason)
    }
}

/// Assess risk for file path operations
///
/// # Arguments
/// * `path` - The file path to assess
///
/// # Returns
/// A `RiskAssessment` based on the file's sensitivity
///
/// # Example
/// ```
/// use zero_core::security::risk::assess_file_risk;
///
/// let result = assess_file_risk("/etc/passwd");
/// assert_eq!(result.risk.as_str(), "high");
///
/// let result = assess_file_risk("src/main.rs");
/// assert_eq!(result.risk.as_str(), "safe");
/// ```
pub fn assess_file_risk(path: &str) -> RiskAssessment {
    // Sensitive file extensions
    if path.ends_with(".env")
        || path.ends_with(".pem")
        || path.ends_with(".key")
        || path.ends_with(".crt")
        || path.ends_with(".p12")
        || path.ends_with(".pfx")
        || path.ends_with(".jks")
        || path.ends_with(".keystore")
    {
        return RiskAssessment::new(
            RiskLevel::High,
            "Sensitive file (credentials/secrets)",
        );
    }

    // Configuration files that might contain secrets
    if path.contains("credentials")
        || path.contains("secrets")
        || path.contains("password")
        || path.contains(".npmrc")
        || path.contains(".netrc")
        || path.contains(".aws/")
        || path.contains(".ssh/")
    {
        return RiskAssessment::new(
            RiskLevel::High,
            "Potential secrets file",
        );
    }

    // System directories
    if path.starts_with("/etc/")
        || path.starts_with("/usr/")
        || path.starts_with("/var/")
        || path.starts_with("/sys/")
        || path.starts_with("/proc/")
        || path.starts_with("/boot/")
    {
        return RiskAssessment::new(
            RiskLevel::High,
            "System directory",
        );
    }

    // Windows system paths
    if path.to_lowercase().starts_with("c:\\windows")
        || path.to_lowercase().starts_with("c:\\program files")
    {
        return RiskAssessment::new(
            RiskLevel::High,
            "System directory (Windows)",
        );
    }

    // Dependency manifests
    if path.ends_with("package.json")
        || path.ends_with("package-lock.json")
        || path.ends_with("Cargo.toml")
        || path.ends_with("Cargo.lock")
        || path.ends_with("go.mod")
        || path.ends_with("go.sum")
        || path.ends_with("requirements.txt")
        || path.ends_with("Pipfile")
        || path.ends_with("pyproject.toml")
        || path.ends_with("Gemfile")
        || path.ends_with("Gemfile.lock")
        || path.ends_with("composer.json")
        || path.ends_with("pom.xml")
        || path.ends_with("build.gradle")
    {
        return RiskAssessment::new(
            RiskLevel::Medium,
            "Dependency manifest",
        );
    }

    // CI/CD configuration
    if path.contains(".github/workflows")
        || path.contains(".gitlab-ci")
        || path.contains("Jenkinsfile")
        || path.contains(".circleci")
        || path.contains("azure-pipelines")
    {
        return RiskAssessment::new(
            RiskLevel::Medium,
            "CI/CD configuration",
        );
    }

    // Docker files
    if path.ends_with("Dockerfile")
        || path.ends_with("docker-compose.yml")
        || path.ends_with("docker-compose.yaml")
    {
        return RiskAssessment::new(
            RiskLevel::Medium,
            "Docker configuration",
        );
    }

    // Normal files
    RiskAssessment::new(RiskLevel::Safe, "Normal file")
}

/// Check if risk is at or below threshold
pub fn risk_at_or_below_threshold(risk: RiskLevel, threshold: RiskLevel) -> bool {
    risk.value() <= threshold.value()
}

// ============================================================================
// Tool Base Risk
// ============================================================================

/// Get base risk level for a tool
///
/// # Arguments
/// * `tool` - The tool name
///
/// # Returns
/// The base risk level for the tool
pub fn tool_base_risk(tool: &str) -> RiskLevel {
    match tool {
        // Safe - No side effects, read-only
        "Read" | "Glob" | "Grep" | "LS" | "NotebookRead" | "TaskList" | "TaskGet" => {
            RiskLevel::Safe
        }

        // Low - External read-only
        "WebFetch" | "WebSearch" => RiskLevel::Low,

        // Medium - Local reversible writes
        "Write" | "Edit" | "NotebookEdit" | "TaskCreate" | "TaskUpdate" => {
            RiskLevel::Medium
        }

        // High - External side effects or semi-reversible
        "Bash" | "Task" => RiskLevel::High,

        // MCP tools (browser automation)
        t if t.starts_with("mcp__playwright__") => RiskLevel::High,

        // Unknown tools default to medium
        _ => RiskLevel::Medium,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Low);
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_parse() {
        assert_eq!(RiskLevel::parse("safe"), Some(RiskLevel::Safe));
        assert_eq!(RiskLevel::parse("SAFE"), Some(RiskLevel::Safe));
        assert_eq!(RiskLevel::parse("Critical"), Some(RiskLevel::Critical));
        assert_eq!(RiskLevel::parse("unknown"), None);
    }

    #[test]
    fn test_bash_risk_critical() {
        assert_eq!(assess_bash_risk("sudo rm -rf /").risk, RiskLevel::Critical);
        assert_eq!(assess_bash_risk("shutdown -h now").risk, RiskLevel::Critical);
        assert_eq!(assess_bash_risk("git push --force").risk, RiskLevel::Critical);
        assert_eq!(assess_bash_risk("rm -rf *").risk, RiskLevel::Critical);
    }

    #[test]
    fn test_bash_risk_high() {
        assert_eq!(assess_bash_risk("git push origin main").risk, RiskLevel::High);
        assert_eq!(assess_bash_risk("rm -rf node_modules").risk, RiskLevel::High);
        assert_eq!(assess_bash_risk("curl -X POST https://api.example.com").risk, RiskLevel::High);
        assert_eq!(assess_bash_risk("npm publish").risk, RiskLevel::High);
    }

    #[test]
    fn test_bash_risk_medium() {
        assert_eq!(assess_bash_risk("git commit -m 'test'").risk, RiskLevel::Medium);
        assert_eq!(assess_bash_risk("npm install lodash").risk, RiskLevel::Medium);
        assert_eq!(assess_bash_risk("mkdir -p src/utils").risk, RiskLevel::Medium);
        assert_eq!(assess_bash_risk("touch README.md").risk, RiskLevel::Medium);
    }

    #[test]
    fn test_bash_risk_low() {
        assert_eq!(assess_bash_risk("git status").risk, RiskLevel::Low);
        assert_eq!(assess_bash_risk("ls -la").risk, RiskLevel::Low);
        assert_eq!(assess_bash_risk("cat file.txt").risk, RiskLevel::Low);
        assert_eq!(assess_bash_risk("grep -r 'pattern' src/").risk, RiskLevel::Low);
    }

    #[test]
    fn test_bash_risk_unknown() {
        // Unknown commands default to high
        assert_eq!(assess_bash_risk("some-custom-script.sh").risk, RiskLevel::High);
    }

    #[test]
    fn test_file_risk_sensitive() {
        assert_eq!(assess_file_risk(".env").risk, RiskLevel::High);
        assert_eq!(assess_file_risk("server.key").risk, RiskLevel::High);
        assert_eq!(assess_file_risk("/etc/passwd").risk, RiskLevel::High);
        assert_eq!(assess_file_risk("~/.ssh/id_rsa").risk, RiskLevel::High);
    }

    #[test]
    fn test_file_risk_medium() {
        assert_eq!(assess_file_risk("package.json").risk, RiskLevel::Medium);
        assert_eq!(assess_file_risk("Cargo.toml").risk, RiskLevel::Medium);
        assert_eq!(assess_file_risk(".github/workflows/ci.yml").risk, RiskLevel::Medium);
    }

    #[test]
    fn test_file_risk_safe() {
        assert_eq!(assess_file_risk("src/main.rs").risk, RiskLevel::Safe);
        assert_eq!(assess_file_risk("README.md").risk, RiskLevel::Safe);
        assert_eq!(assess_file_risk("test/unit.test.ts").risk, RiskLevel::Safe);
    }

    #[test]
    fn test_tool_base_risk() {
        assert_eq!(tool_base_risk("Read"), RiskLevel::Safe);
        assert_eq!(tool_base_risk("Glob"), RiskLevel::Safe);
        assert_eq!(tool_base_risk("WebFetch"), RiskLevel::Low);
        assert_eq!(tool_base_risk("Write"), RiskLevel::Medium);
        assert_eq!(tool_base_risk("Bash"), RiskLevel::High);
        assert_eq!(tool_base_risk("mcp__playwright__browser_click"), RiskLevel::High);
        assert_eq!(tool_base_risk("UnknownTool"), RiskLevel::Medium);
    }

    #[test]
    fn test_risk_threshold() {
        assert!(risk_at_or_below_threshold(RiskLevel::Safe, RiskLevel::Low));
        assert!(risk_at_or_below_threshold(RiskLevel::Low, RiskLevel::Low));
        assert!(!risk_at_or_below_threshold(RiskLevel::Medium, RiskLevel::Low));
        assert!(risk_at_or_below_threshold(RiskLevel::High, RiskLevel::Critical));
    }
}
