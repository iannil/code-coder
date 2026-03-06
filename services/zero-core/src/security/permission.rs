//! Permission management

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

/// A permission that can be granted or denied
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Tool name
    pub tool: String,
    /// Action pattern (can include wildcards)
    pub action: String,
    /// Resource pattern (can include wildcards)
    pub resource: Option<String>,
}

impl Permission {
    /// Create a new permission
    pub fn new(tool: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            action: action.into(),
            resource: None,
        }
    }

    /// Create a permission with a resource
    pub fn with_resource(tool: impl Into<String>, action: impl Into<String>, resource: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            action: action.into(),
            resource: Some(resource.into()),
        }
    }

    /// Check if this permission matches another
    pub fn matches(&self, other: &Permission) -> bool {
        self.matches_pattern(&self.tool, &other.tool)
            && self.matches_pattern(&self.action, &other.action)
            && match (&self.resource, &other.resource) {
                (Some(a), Some(b)) => self.matches_pattern(a, b),
                (None, _) => true,
                (Some(_), None) => false,
            }
    }

    /// Match a pattern with wildcards
    fn matches_pattern(&self, pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return value.starts_with(prefix);
        }
        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            return value.ends_with(suffix);
        }
        pattern == value
    }
}

/// A permission rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Permission being granted
    pub permission: Permission,
    /// Whether this is an allow or deny rule
    pub allow: bool,
    /// Description of why this permission is needed
    pub reason: Option<String>,
}

impl PermissionRule {
    /// Create an allow rule
    pub fn allow(permission: Permission) -> Self {
        Self {
            permission,
            allow: true,
            reason: None,
        }
    }

    /// Create a deny rule
    pub fn deny(permission: Permission) -> Self {
        Self {
            permission,
            allow: false,
            reason: None,
        }
    }

    /// Add a reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Permission manager
#[derive(Debug, Default)]
pub struct PermissionManager {
    /// Permission rules (evaluated in order)
    rules: Vec<PermissionRule>,
    /// Granted permissions (cached)
    granted: HashSet<String>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    /// Grant a permission
    pub fn grant(&mut self, permission: &Permission) {
        let key = format!("{}:{}:{}", permission.tool, permission.action, permission.resource.as_deref().unwrap_or("*"));
        self.granted.insert(key);
    }

    /// Check if a permission is allowed
    pub fn check(&self, permission: &Permission) -> bool {
        // Check cached grants first
        let key = format!("{}:{}:{}", permission.tool, permission.action, permission.resource.as_deref().unwrap_or("*"));
        if self.granted.contains(&key) {
            return true;
        }

        // Check rules in reverse order (last rule wins)
        for rule in self.rules.iter().rev() {
            if rule.permission.matches(permission) {
                return rule.allow;
            }
        }

        // Default deny
        false
    }

    /// Check and request permission (returns whether to proceed)
    pub fn check_or_request(&mut self, permission: &Permission) -> PermissionResult {
        if self.check(permission) {
            PermissionResult::Allowed
        } else {
            PermissionResult::NeedsApproval(permission.clone())
        }
    }
}

/// Result of a permission check
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Permission is allowed
    Allowed,
    /// Permission needs user approval
    NeedsApproval(Permission),
    /// Permission is denied
    Denied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_matching() {
        let rule = Permission::new("bash", "*");
        let requested = Permission::new("bash", "execute");
        assert!(rule.matches(&requested));

        let rule = Permission::new("file", "read");
        let requested = Permission::new("file", "write");
        assert!(!rule.matches(&requested));
    }

    #[test]
    fn test_permission_manager() {
        let mut manager = PermissionManager::new();

        // Allow all read operations
        manager.add_rule(PermissionRule::allow(Permission::new("file", "read")));

        // Deny writes to /etc
        manager.add_rule(PermissionRule::deny(
            Permission::with_resource("file", "write", "/etc/*")
        ));

        assert!(manager.check(&Permission::new("file", "read")));
        assert!(!manager.check(&Permission::with_resource("file", "write", "/etc/passwd")));
        assert!(!manager.check(&Permission::new("bash", "execute"))); // Not explicitly allowed
    }

    #[test]
    fn test_grant_permission() {
        let mut manager = PermissionManager::new();

        let permission = Permission::new("bash", "execute");
        assert!(!manager.check(&permission));

        manager.grant(&permission);
        assert!(manager.check(&permission));
    }
}
