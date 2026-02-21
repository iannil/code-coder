//! Resource-based RBAC with wildcard pattern matching.
//!
//! Extends the basic RBAC system to support dynamic resources like agents and skills.
//!
//! # Pattern Format
//!
//! Permissions use the format: `<resource_type>:<resource_id>:<action>`
//!
//! Examples:
//! - `agent:*:execute` - Execute any agent
//! - `agent:code-reviewer:execute` - Execute specific agent
//! - `skill:prd-*:read` - Read any skill starting with "prd-"
//! - `workflow:*:*` - Full access to all workflows
//!
//! # Supported Resource Types
//!
//! - `agent` - AI agents (code-reviewer, planner, etc.)
//! - `skill` - Skills (tdd, e2e, etc.)
//! - `workflow` - Workflow definitions
//! - `prompt` - Prompt templates
//! - `channel` - Communication channels (telegram, slack, etc.)
//! - `provider` - AI providers (anthropic, openai, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Actions that can be performed on resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Read resource metadata or configuration
    Read,
    /// Write or modify resource
    Write,
    /// Execute resource (for agents, workflows)
    Execute,
    /// Delete resource
    Delete,
    /// Full administrative access
    Admin,
}

impl Action {
    /// Parse an action from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read" | "r" => Some(Action::Read),
            "write" | "w" => Some(Action::Write),
            "execute" | "exec" | "x" => Some(Action::Execute),
            "delete" | "d" => Some(Action::Delete),
            "admin" | "*" => Some(Action::Admin),
            _ => None,
        }
    }

    /// Check if this action implies another action.
    /// Admin implies all other actions.
    pub fn implies(&self, other: &Action) -> bool {
        *self == Action::Admin || *self == *other
    }
}

/// Resource type categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Agent,
    Skill,
    Workflow,
    Prompt,
    Channel,
    Provider,
    Memory,
    Config,
    Custom(String),
}

impl ResourceType {
    /// Parse a resource type from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "agent" => ResourceType::Agent,
            "skill" => ResourceType::Skill,
            "workflow" => ResourceType::Workflow,
            "prompt" => ResourceType::Prompt,
            "channel" => ResourceType::Channel,
            "provider" => ResourceType::Provider,
            "memory" => ResourceType::Memory,
            "config" => ResourceType::Config,
            other => ResourceType::Custom(other.to_string()),
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &str {
        match self {
            ResourceType::Agent => "agent",
            ResourceType::Skill => "skill",
            ResourceType::Workflow => "workflow",
            ResourceType::Prompt => "prompt",
            ResourceType::Channel => "channel",
            ResourceType::Provider => "provider",
            ResourceType::Memory => "memory",
            ResourceType::Config => "config",
            ResourceType::Custom(s) => s,
        }
    }
}

/// A resource permission pattern with wildcard support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermission {
    /// Resource type (agent, skill, workflow, etc.)
    pub resource_type: ResourceType,
    /// Resource pattern (* for all, prefix-* for prefix matching)
    pub resource_pattern: String,
    /// Allowed actions
    pub actions: HashSet<Action>,
}

impl ResourcePermission {
    /// Create a new resource permission.
    pub fn new(resource_type: ResourceType, pattern: &str, actions: &[Action]) -> Self {
        Self {
            resource_type,
            resource_pattern: pattern.to_string(),
            actions: actions.iter().cloned().collect(),
        }
    }

    /// Create a permission with full access to a resource type.
    pub fn full_access(resource_type: ResourceType) -> Self {
        Self::new(resource_type, "*", &[Action::Admin])
    }

    /// Parse a permission from string format: `type:pattern:actions`.
    /// Actions can be comma-separated: `agent:*:read,execute`
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 2 {
            return None;
        }

        let resource_type = ResourceType::from_str(parts[0]);
        let pattern = parts[1].to_string();

        let actions: HashSet<Action> = if parts.len() >= 3 {
            parts[2]
                .split(',')
                .filter_map(|a| Action::from_str(a.trim()))
                .collect()
        } else {
            // Default to all actions if not specified
            [Action::Admin].into_iter().collect()
        };

        if actions.is_empty() {
            return None;
        }

        Some(Self {
            resource_type,
            resource_pattern: pattern,
            actions,
        })
    }

    /// Check if this permission matches a specific resource and action.
    pub fn matches(&self, resource_type: &ResourceType, resource_id: &str, action: &Action) -> bool {
        // Check resource type
        if &self.resource_type != resource_type {
            return false;
        }

        // Check action (Admin implies all)
        let action_allowed = self.actions.iter().any(|a| a.implies(action));
        if !action_allowed {
            return false;
        }

        // Check resource pattern
        self.pattern_matches(resource_id)
    }

    /// Check if the pattern matches a resource ID.
    fn pattern_matches(&self, resource_id: &str) -> bool {
        let pattern = &self.resource_pattern;

        // Exact wildcard match
        if pattern == "*" {
            return true;
        }

        // Prefix wildcard (e.g., "prd-*")
        if let Some(prefix) = pattern.strip_suffix('*') {
            return resource_id.starts_with(prefix);
        }

        // Suffix wildcard (e.g., "*-reviewer")
        if let Some(suffix) = pattern.strip_prefix('*') {
            return resource_id.ends_with(suffix);
        }

        // Exact match
        pattern == resource_id
    }

    /// Convert to string format.
    pub fn to_string(&self) -> String {
        let actions: Vec<&str> = self
            .actions
            .iter()
            .map(|a| match a {
                Action::Read => "read",
                Action::Write => "write",
                Action::Execute => "execute",
                Action::Delete => "delete",
                Action::Admin => "*",
            })
            .collect();
        format!(
            "{}:{}:{}",
            self.resource_type.as_str(),
            self.resource_pattern,
            actions.join(",")
        )
    }
}

/// A role with resource-based permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRole {
    /// Role name
    pub name: String,
    /// Role description
    pub description: String,
    /// Resource permissions
    pub permissions: Vec<ResourcePermission>,
}

impl ResourceRole {
    /// Create a new empty role.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            permissions: Vec::new(),
        }
    }

    /// Add a permission to this role.
    pub fn with_permission(mut self, permission: ResourcePermission) -> Self {
        self.permissions.push(permission);
        self
    }

    /// Check if this role grants access to a resource.
    pub fn can_access(&self, resource_type: &ResourceType, resource_id: &str, action: &Action) -> bool {
        self.permissions
            .iter()
            .any(|p| p.matches(resource_type, resource_id, action))
    }
}

/// Predefined resource roles.
pub mod resource_roles {
    use super::*;

    /// Super admin with full access to everything.
    pub fn super_admin() -> ResourceRole {
        ResourceRole::new("super_admin", "Full access to all resources")
            .with_permission(ResourcePermission::full_access(ResourceType::Agent))
            .with_permission(ResourcePermission::full_access(ResourceType::Skill))
            .with_permission(ResourcePermission::full_access(ResourceType::Workflow))
            .with_permission(ResourcePermission::full_access(ResourceType::Prompt))
            .with_permission(ResourcePermission::full_access(ResourceType::Channel))
            .with_permission(ResourcePermission::full_access(ResourceType::Provider))
            .with_permission(ResourcePermission::full_access(ResourceType::Memory))
            .with_permission(ResourcePermission::full_access(ResourceType::Config))
    }

    /// Developer role with agent and skill execution access.
    pub fn developer() -> ResourceRole {
        ResourceRole::new("developer", "Execute agents and skills, read workflows")
            .with_permission(ResourcePermission::new(
                ResourceType::Agent,
                "*",
                &[Action::Read, Action::Execute],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Skill,
                "*",
                &[Action::Read, Action::Execute],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Workflow,
                "*",
                &[Action::Read],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Prompt,
                "*",
                &[Action::Read],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Provider,
                "*",
                &[Action::Read, Action::Execute],
            ))
    }

    /// Viewer role with read-only access.
    pub fn viewer() -> ResourceRole {
        ResourceRole::new("viewer", "Read-only access to resources")
            .with_permission(ResourcePermission::new(
                ResourceType::Agent,
                "*",
                &[Action::Read],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Skill,
                "*",
                &[Action::Read],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Workflow,
                "*",
                &[Action::Read],
            ))
    }

    /// Workflow automation role for CI/CD pipelines.
    pub fn automation() -> ResourceRole {
        ResourceRole::new("automation", "Execute workflows and agents for automation")
            .with_permission(ResourcePermission::new(
                ResourceType::Agent,
                "code-reviewer",
                &[Action::Execute],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Agent,
                "security-reviewer",
                &[Action::Execute],
            ))
            .with_permission(ResourcePermission::new(
                ResourceType::Workflow,
                "*",
                &[Action::Read, Action::Execute],
            ))
    }

    /// Get a resource role by name.
    pub fn get_by_name(name: &str) -> Option<ResourceRole> {
        match name {
            "super_admin" => Some(super_admin()),
            "developer" => Some(developer()),
            "viewer" => Some(viewer()),
            "automation" => Some(automation()),
            _ => None,
        }
    }

    /// List all predefined role names.
    pub fn all_names() -> &'static [&'static str] {
        &["super_admin", "developer", "viewer", "automation"]
    }
}

/// Check if a user with the given roles can access a resource.
pub fn can_access_resource(
    role_names: &[String],
    resource_type: &ResourceType,
    resource_id: &str,
    action: &Action,
) -> bool {
    role_names.iter().any(|name| {
        resource_roles::get_by_name(name)
            .map(|role| role.can_access(resource_type, resource_id, action))
            .unwrap_or(false)
    })
}

/// Check multiple resource accesses at once.
pub fn can_access_all(
    role_names: &[String],
    requests: &[(ResourceType, String, Action)],
) -> bool {
    requests
        .iter()
        .all(|(rt, rid, a)| can_access_resource(role_names, rt, rid, a))
}

/// Dynamic role store for custom roles.
pub struct ResourceRoleStore {
    roles: std::sync::RwLock<std::collections::HashMap<String, ResourceRole>>,
}

impl ResourceRoleStore {
    /// Create a new role store with predefined roles.
    pub fn new() -> Self {
        let mut roles = std::collections::HashMap::new();
        for name in resource_roles::all_names() {
            if let Some(role) = resource_roles::get_by_name(name) {
                roles.insert(name.to_string(), role);
            }
        }
        Self {
            roles: std::sync::RwLock::new(roles),
        }
    }

    /// Get a role by name.
    pub fn get(&self, name: &str) -> Option<ResourceRole> {
        self.roles
            .read()
            .ok()
            .and_then(|r| r.get(name).cloned())
    }

    /// Create or update a role.
    pub fn upsert(&self, role: ResourceRole) -> Result<(), &'static str> {
        let mut roles = self.roles.write().map_err(|_| "Lock poisoned")?;
        roles.insert(role.name.clone(), role);
        Ok(())
    }

    /// Delete a role.
    pub fn delete(&self, name: &str) -> Result<bool, &'static str> {
        // Prevent deleting predefined roles
        if resource_roles::all_names().contains(&name) {
            return Err("Cannot delete predefined role");
        }
        let mut roles = self.roles.write().map_err(|_| "Lock poisoned")?;
        Ok(roles.remove(name).is_some())
    }

    /// List all role names.
    pub fn list(&self) -> Vec<String> {
        self.roles
            .read()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if a user can access a resource.
    pub fn can_access(
        &self,
        role_names: &[String],
        resource_type: &ResourceType,
        resource_id: &str,
        action: &Action,
    ) -> bool {
        let roles = match self.roles.read() {
            Ok(r) => r,
            Err(_) => return false,
        };

        role_names.iter().any(|name| {
            roles
                .get(name)
                .map(|role| role.can_access(resource_type, resource_id, action))
                .unwrap_or(false)
        })
    }
}

impl Default for ResourceRoleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_wildcard() {
        let perm = ResourcePermission::new(ResourceType::Agent, "*", &[Action::Execute]);

        assert!(perm.matches(&ResourceType::Agent, "code-reviewer", &Action::Execute));
        assert!(perm.matches(&ResourceType::Agent, "any-agent", &Action::Execute));
        assert!(!perm.matches(&ResourceType::Agent, "code-reviewer", &Action::Delete));
        assert!(!perm.matches(&ResourceType::Skill, "tdd", &Action::Execute));
    }

    #[test]
    fn test_permission_prefix_wildcard() {
        let perm = ResourcePermission::new(ResourceType::Skill, "prd-*", &[Action::Read]);

        assert!(perm.matches(&ResourceType::Skill, "prd-analysis", &Action::Read));
        assert!(perm.matches(&ResourceType::Skill, "prd-template", &Action::Read));
        assert!(!perm.matches(&ResourceType::Skill, "tdd", &Action::Read));
        assert!(!perm.matches(&ResourceType::Skill, "analysis-prd", &Action::Read));
    }

    #[test]
    fn test_permission_exact_match() {
        let perm = ResourcePermission::new(ResourceType::Agent, "code-reviewer", &[Action::Execute]);

        assert!(perm.matches(&ResourceType::Agent, "code-reviewer", &Action::Execute));
        assert!(!perm.matches(&ResourceType::Agent, "security-reviewer", &Action::Execute));
    }

    #[test]
    fn test_admin_implies_all() {
        let perm = ResourcePermission::new(ResourceType::Agent, "*", &[Action::Admin]);

        assert!(perm.matches(&ResourceType::Agent, "any", &Action::Read));
        assert!(perm.matches(&ResourceType::Agent, "any", &Action::Write));
        assert!(perm.matches(&ResourceType::Agent, "any", &Action::Execute));
        assert!(perm.matches(&ResourceType::Agent, "any", &Action::Delete));
    }

    #[test]
    fn test_permission_parse() {
        let perm = ResourcePermission::parse("agent:*:read,execute").unwrap();
        assert!(perm.matches(&ResourceType::Agent, "test", &Action::Read));
        assert!(perm.matches(&ResourceType::Agent, "test", &Action::Execute));
        assert!(!perm.matches(&ResourceType::Agent, "test", &Action::Delete));
    }

    #[test]
    fn test_super_admin_role() {
        let role = resource_roles::super_admin();

        assert!(role.can_access(&ResourceType::Agent, "any", &Action::Admin));
        assert!(role.can_access(&ResourceType::Skill, "any", &Action::Delete));
        assert!(role.can_access(&ResourceType::Workflow, "any", &Action::Execute));
    }

    #[test]
    fn test_developer_role() {
        let role = resource_roles::developer();

        assert!(role.can_access(&ResourceType::Agent, "any", &Action::Execute));
        assert!(role.can_access(&ResourceType::Agent, "any", &Action::Read));
        assert!(!role.can_access(&ResourceType::Agent, "any", &Action::Delete));
        assert!(role.can_access(&ResourceType::Workflow, "any", &Action::Read));
        assert!(!role.can_access(&ResourceType::Workflow, "any", &Action::Write));
    }

    #[test]
    fn test_viewer_role() {
        let role = resource_roles::viewer();

        assert!(role.can_access(&ResourceType::Agent, "any", &Action::Read));
        assert!(!role.can_access(&ResourceType::Agent, "any", &Action::Execute));
        assert!(!role.can_access(&ResourceType::Agent, "any", &Action::Write));
    }

    #[test]
    fn test_can_access_resource() {
        let roles = vec!["developer".to_string()];

        assert!(can_access_resource(
            &roles,
            &ResourceType::Agent,
            "code-reviewer",
            &Action::Execute
        ));
        assert!(!can_access_resource(
            &roles,
            &ResourceType::Agent,
            "code-reviewer",
            &Action::Delete
        ));
    }

    #[test]
    fn test_role_store() {
        let store = ResourceRoleStore::new();

        // Predefined roles should be available
        assert!(store.get("developer").is_some());
        assert!(store.get("super_admin").is_some());

        // Custom role
        let custom = ResourceRole::new("custom_role", "Custom test role")
            .with_permission(ResourcePermission::new(
                ResourceType::Agent,
                "test-*",
                &[Action::Execute],
            ));

        store.upsert(custom).unwrap();
        assert!(store.get("custom_role").is_some());

        // Check access with custom role
        assert!(store.can_access(
            &["custom_role".to_string()],
            &ResourceType::Agent,
            "test-agent",
            &Action::Execute
        ));
        assert!(!store.can_access(
            &["custom_role".to_string()],
            &ResourceType::Agent,
            "other-agent",
            &Action::Execute
        ));

        // Cannot delete predefined roles
        assert!(store.delete("developer").is_err());

        // Can delete custom roles
        assert!(store.delete("custom_role").unwrap());
        assert!(store.get("custom_role").is_none());
    }

    #[test]
    fn test_permission_to_string() {
        let perm = ResourcePermission::new(
            ResourceType::Agent,
            "code-*",
            &[Action::Read, Action::Execute],
        );
        let s = perm.to_string();
        assert!(s.contains("agent:code-*:"));
        assert!(s.contains("read"));
        assert!(s.contains("execute"));
    }
}
