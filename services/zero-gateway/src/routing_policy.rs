//! Provider routing policy for Zero Gateway.
//!
//! Provides intelligent routing decisions based on content sensitivity.
//! When sensitive data is detected, requests can be forced to use private/local models
//! instead of cloud providers for data compliance.
//!
//! # Configuration
//!
//! ```json
//! {
//!   "routing_policy": {
//!     "enabled": true,
//!     "default_provider": "anthropic",
//!     "private_provider": "ollama",
//!     "force_private_patterns": ["pii_*", "credit_card", "private_key"],
//!     "sensitivity_threshold": "high"
//!   }
//! }
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Sensitivity level for content classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensitivityLevel {
    /// No sensitive data detected
    None,
    /// Low sensitivity (general business data)
    Low,
    /// Medium sensitivity (internal documents, non-critical PII)
    Medium,
    /// High sensitivity (credentials, financial data, health records)
    High,
    /// Critical sensitivity (must never leave organization)
    Critical,
}

impl Default for SensitivityLevel {
    fn default() -> Self {
        Self::None
    }
}

impl std::str::FromStr for SensitivityLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" | "crit" => Ok(Self::Critical),
            _ => Err(format!("Unknown sensitivity level: {}", s)),
        }
    }
}

/// Routing decision for a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Whether to use private model
    pub use_private: bool,
    /// Recommended provider name
    pub provider: String,
    /// Reason for the decision
    pub reason: String,
    /// Detected sensitivity level
    pub sensitivity: SensitivityLevel,
    /// Patterns that triggered the decision
    pub triggered_patterns: Vec<String>,
}

impl RoutingDecision {
    /// Create a decision to use the default provider.
    pub fn use_default(provider: &str) -> Self {
        Self {
            use_private: false,
            provider: provider.to_string(),
            reason: "No sensitive data detected".to_string(),
            sensitivity: SensitivityLevel::None,
            triggered_patterns: vec![],
        }
    }

    /// Create a decision to force private model.
    pub fn force_private(provider: &str, reason: &str, sensitivity: SensitivityLevel, patterns: Vec<String>) -> Self {
        Self {
            use_private: true,
            provider: provider.to_string(),
            reason: reason.to_string(),
            sensitivity,
            triggered_patterns: patterns,
        }
    }
}

/// Sensitive pattern with associated sensitivity level.
#[derive(Debug, Clone)]
pub struct SensitivePatternRule {
    /// Pattern name (for logging and matching)
    pub name: String,
    /// Regex pattern
    pub pattern: Regex,
    /// Sensitivity level when matched
    pub level: SensitivityLevel,
    /// Human-readable description
    pub description: String,
}

impl SensitivePatternRule {
    /// Create a new pattern rule.
    pub fn new(name: &str, pattern: &str, level: SensitivityLevel, description: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            name: name.to_string(),
            pattern: Regex::new(pattern)?,
            level,
            description: description.to_string(),
        })
    }
}

/// Routing policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicyConfig {
    /// Enable routing policy
    pub enabled: bool,
    /// Default cloud provider
    pub default_provider: String,
    /// Private/local provider for sensitive data
    pub private_provider: String,
    /// Minimum sensitivity level to trigger private routing
    pub sensitivity_threshold: SensitivityLevel,
    /// Force private routing for specific pattern names (supports wildcards)
    pub force_private_patterns: Vec<String>,
    /// Allow list of users who can bypass policy (for testing)
    pub bypass_users: Vec<String>,
}

impl Default for RoutingPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_provider: "anthropic".to_string(),
            private_provider: "ollama".to_string(),
            sensitivity_threshold: SensitivityLevel::High,
            force_private_patterns: vec![
                "credit_card".to_string(),
                "private_key".to_string(),
                "aws_*".to_string(),
            ],
            bypass_users: vec![],
        }
    }
}

/// Provider routing policy.
pub struct RoutingPolicy {
    config: RoutingPolicyConfig,
    patterns: Vec<SensitivePatternRule>,
}

impl RoutingPolicy {
    /// Create a new routing policy with default patterns.
    pub fn new(config: RoutingPolicyConfig) -> Self {
        let patterns = Self::default_patterns();
        Self { config, patterns }
    }

    /// Create default sensitive patterns.
    fn default_patterns() -> Vec<SensitivePatternRule> {
        vec![
            // ── Critical Level ──────────────────────────────────────────
            SensitivePatternRule::new(
                "private_key",
                r"-----BEGIN[A-Z ]*PRIVATE KEY-----",
                SensitivityLevel::Critical,
                "Private cryptographic key",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "aws_secret_key",
                r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key["']?\s*[:=]\s*["']?([A-Za-z0-9/+=]{40})"#,
                SensitivityLevel::Critical,
                "AWS secret access key",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "database_url_password",
                r"(?i)(postgres|mysql|mongodb)://[^:]+:([^@]+)@",
                SensitivityLevel::Critical,
                "Database connection with password",
            )
            .unwrap(),
            // ── High Level ──────────────────────────────────────────────
            SensitivePatternRule::new(
                "credit_card",
                r"\b(?:\d{4}[- ]?){3}\d{4}\b",
                SensitivityLevel::High,
                "Credit card number",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "ssn",
                r"\b\d{3}-\d{2}-\d{4}\b",
                SensitivityLevel::High,
                "Social Security Number",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "anthropic_key",
                r"sk-ant-[a-zA-Z0-9\-]{95,}",
                SensitivityLevel::High,
                "Anthropic API key",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "openai_key",
                r"sk-[a-zA-Z0-9]{48}",
                SensitivityLevel::High,
                "OpenAI API key",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "aws_access_key",
                r"AKIA[0-9A-Z]{16}",
                SensitivityLevel::High,
                "AWS access key ID",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "jwt_token",
                r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]+",
                SensitivityLevel::High,
                "JWT token",
            )
            .unwrap(),
            // ── Medium Level ────────────────────────────────────────────
            SensitivePatternRule::new(
                "email_pii",
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
                SensitivityLevel::Medium,
                "Email address (PII)",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "phone_number",
                r"\b(?:\+?1[-.]?)?\(?[0-9]{3}\)?[-.]?[0-9]{3}[-.]?[0-9]{4}\b",
                SensitivityLevel::Medium,
                "Phone number",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "ip_address",
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b",
                SensitivityLevel::Medium,
                "IP address",
            )
            .unwrap(),
            // ── Low Level ───────────────────────────────────────────────
            SensitivePatternRule::new(
                "internal_url",
                r"https?://(?:localhost|127\.0\.0\.1|10\.|172\.1[6-9]\.|172\.2[0-9]\.|172\.3[0-1]\.|192\.168\.)",
                SensitivityLevel::Low,
                "Internal/private URL",
            )
            .unwrap(),
            SensitivePatternRule::new(
                "file_path_secrets",
                r"(?i)(/etc/passwd|/etc/shadow|\.env|credentials\.json|secrets\.yaml)",
                SensitivityLevel::Low,
                "Sensitive file path reference",
            )
            .unwrap(),
        ]
    }

    /// Add a custom pattern rule.
    pub fn add_pattern(&mut self, rule: SensitivePatternRule) {
        self.patterns.push(rule);
    }

    /// Analyze content and return routing decision.
    pub fn analyze(&self, content: &str, user_id: Option<&str>) -> RoutingDecision {
        // Check if disabled or user is bypassed
        if !self.config.enabled {
            return RoutingDecision::use_default(&self.config.default_provider);
        }

        if let Some(uid) = user_id {
            if self.config.bypass_users.contains(&uid.to_string()) {
                return RoutingDecision::use_default(&self.config.default_provider);
            }
        }

        // Detect sensitive patterns
        let mut max_level = SensitivityLevel::None;
        let mut triggered_patterns = Vec::new();

        for rule in &self.patterns {
            if rule.pattern.is_match(content) {
                triggered_patterns.push(rule.name.clone());
                if rule.level > max_level {
                    max_level = rule.level;
                }
            }
        }

        // Check if any triggered pattern is in force_private list
        let force_private = triggered_patterns.iter().any(|p| {
            self.config.force_private_patterns.iter().any(|fp| {
                if fp.ends_with('*') {
                    let prefix = &fp[..fp.len() - 1];
                    p.starts_with(prefix)
                } else {
                    p == fp
                }
            })
        });

        // Decide based on sensitivity threshold or force_private
        if force_private || max_level >= self.config.sensitivity_threshold {
            let reason = if force_private {
                format!(
                    "Forced private routing for patterns: {:?}",
                    triggered_patterns
                )
            } else {
                format!(
                    "Sensitivity level {:?} exceeds threshold {:?}",
                    max_level, self.config.sensitivity_threshold
                )
            };

            RoutingDecision::force_private(
                &self.config.private_provider,
                &reason,
                max_level,
                triggered_patterns,
            )
        } else {
            let mut decision = RoutingDecision::use_default(&self.config.default_provider);
            decision.sensitivity = max_level;
            decision.triggered_patterns = triggered_patterns;
            decision
        }
    }

    /// Quick check if content contains any sensitive data.
    pub fn has_sensitive_data(&self, content: &str) -> bool {
        self.patterns.iter().any(|r| r.pattern.is_match(content))
    }

    /// Get the highest sensitivity level in content.
    pub fn get_sensitivity(&self, content: &str) -> SensitivityLevel {
        self.patterns
            .iter()
            .filter(|r| r.pattern.is_match(content))
            .map(|r| r.level)
            .max()
            .unwrap_or(SensitivityLevel::None)
    }

    /// List all detected patterns in content.
    pub fn detect_patterns(&self, content: &str) -> Vec<&SensitivePatternRule> {
        self.patterns
            .iter()
            .filter(|r| r.pattern.is_match(content))
            .collect()
    }
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self::new(RoutingPolicyConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy() -> RoutingPolicy {
        RoutingPolicy::default()
    }

    #[test]
    fn test_no_sensitive_data() {
        let policy = create_test_policy();
        let decision = policy.analyze("Hello, this is a normal message.", None);

        assert!(!decision.use_private);
        assert_eq!(decision.sensitivity, SensitivityLevel::None);
        assert!(decision.triggered_patterns.is_empty());
    }

    #[test]
    fn test_credit_card_detection() {
        let policy = create_test_policy();
        let decision = policy.analyze("My card number is 4111-1111-1111-1111", None);

        assert!(decision.use_private);
        assert!(decision.triggered_patterns.contains(&"credit_card".to_string()));
        assert!(decision.sensitivity >= SensitivityLevel::High);
    }

    #[test]
    fn test_private_key_detection() {
        let policy = create_test_policy();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpA...";
        let decision = policy.analyze(content, None);

        assert!(decision.use_private);
        assert_eq!(decision.sensitivity, SensitivityLevel::Critical);
        assert!(decision.triggered_patterns.contains(&"private_key".to_string()));
    }

    #[test]
    fn test_aws_key_detection() {
        let policy = create_test_policy();
        let decision = policy.analyze("AWS Key: AKIAIOSFODNN7EXAMPLE", None);

        assert!(decision.use_private);
        assert!(decision.triggered_patterns.contains(&"aws_access_key".to_string()));
    }

    #[test]
    fn test_jwt_detection() {
        let policy = create_test_policy();
        let content =
            "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let decision = policy.analyze(content, None);

        assert!(decision.use_private);
        assert!(decision.triggered_patterns.contains(&"jwt_token".to_string()));
    }

    #[test]
    fn test_email_medium_sensitivity() {
        let policy = create_test_policy();
        let decision = policy.analyze("Contact me at user@example.com", None);

        // Email is medium sensitivity, below default threshold
        assert!(!decision.use_private);
        assert_eq!(decision.sensitivity, SensitivityLevel::Medium);
    }

    #[test]
    fn test_bypass_user() {
        let mut config = RoutingPolicyConfig::default();
        config.bypass_users = vec!["admin".to_string()];
        let policy = RoutingPolicy::new(config);

        // Even with sensitive data, bypassed user goes to default
        let decision = policy.analyze("Card: 4111-1111-1111-1111", Some("admin"));
        assert!(!decision.use_private);

        // Non-bypassed user still gets private routing
        let decision = policy.analyze("Card: 4111-1111-1111-1111", Some("regular"));
        assert!(decision.use_private);
    }

    #[test]
    fn test_disabled_policy() {
        let config = RoutingPolicyConfig {
            enabled: false,
            ..Default::default()
        };
        let policy = RoutingPolicy::new(config);

        let decision = policy.analyze("-----BEGIN RSA PRIVATE KEY-----", None);
        assert!(!decision.use_private);
    }

    #[test]
    fn test_force_private_wildcard() {
        let mut config = RoutingPolicyConfig::default();
        config.force_private_patterns = vec!["aws_*".to_string()];
        config.sensitivity_threshold = SensitivityLevel::Critical; // Higher threshold
        let policy = RoutingPolicy::new(config);

        // AWS key matches force pattern, even though High < Critical threshold
        let decision = policy.analyze("Key: AKIAIOSFODNN7EXAMPLE", None);
        assert!(decision.use_private);
        assert!(decision.reason.contains("Forced"));
    }

    #[test]
    fn test_routing_decision_provider() {
        let config = RoutingPolicyConfig {
            default_provider: "anthropic".to_string(),
            private_provider: "ollama".to_string(),
            ..Default::default()
        };
        let policy = RoutingPolicy::new(config);

        // Normal content → default provider
        let decision = policy.analyze("Hello world", None);
        assert_eq!(decision.provider, "anthropic");

        // Sensitive content → private provider
        let decision = policy.analyze("Card: 4111-1111-1111-1111", None);
        assert_eq!(decision.provider, "ollama");
    }

    #[test]
    fn test_multiple_patterns() {
        let policy = create_test_policy();
        let content = "Email: test@example.com\nCard: 4111-1111-1111-1111\nSSN: 123-45-6789";
        let decision = policy.analyze(content, None);

        assert!(decision.use_private);
        assert!(decision.triggered_patterns.len() >= 3);
        assert!(decision.triggered_patterns.contains(&"email_pii".to_string()));
        assert!(decision.triggered_patterns.contains(&"credit_card".to_string()));
        assert!(decision.triggered_patterns.contains(&"ssn".to_string()));
    }

    #[test]
    fn test_sensitivity_level_ordering() {
        assert!(SensitivityLevel::None < SensitivityLevel::Low);
        assert!(SensitivityLevel::Low < SensitivityLevel::Medium);
        assert!(SensitivityLevel::Medium < SensitivityLevel::High);
        assert!(SensitivityLevel::High < SensitivityLevel::Critical);
    }

    #[test]
    fn test_has_sensitive_data() {
        let policy = create_test_policy();

        assert!(!policy.has_sensitive_data("Normal text"));
        assert!(policy.has_sensitive_data("Card: 4111-1111-1111-1111"));
        assert!(policy.has_sensitive_data("AWS Key: AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_detect_patterns() {
        let policy = create_test_policy();
        let patterns = policy.detect_patterns("Email: test@example.com, IP: 192.168.1.1");

        assert!(!patterns.is_empty());
        let names: Vec<_> = patterns.iter().map(|p| &p.name).collect();
        assert!(names.contains(&&"email_pii".to_string()));
        assert!(names.contains(&&"ip_address".to_string()));
    }
}
