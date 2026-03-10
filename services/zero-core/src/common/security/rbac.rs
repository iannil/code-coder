//! Role-Based Access Control (RBAC) for Zero services.
//!
//! Provides a permission system with predefined roles:
//! - `admin`: Full access to all resources
//! - `user`: Basic usage (proxy access, read own quota)
//! - `readonly`: Read-only access

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Permissions that can be granted to users.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read user information
    UserRead,
    /// Create/update/delete users
    UserWrite,
    /// Manage user roles and permissions
    UserAdmin,
    /// Read quota information
    QuotaRead,
    /// Modify quota limits
    QuotaWrite,
    /// Access proxy endpoints
    ProxyAccess,
    /// Access admin dashboard and management
    AdminAccess,
    /// View audit logs
    AuditRead,
    /// Manage system configuration
    ConfigWrite,
}

impl Permission {
    /// Get all available permissions.
    pub fn all() -> &'static [Permission] {
        &[
            Permission::UserRead,
            Permission::UserWrite,
            Permission::UserAdmin,
            Permission::QuotaRead,
            Permission::QuotaWrite,
            Permission::ProxyAccess,
            Permission::AdminAccess,
            Permission::AuditRead,
            Permission::ConfigWrite,
        ]
    }
}

/// A role with associated permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Role description
    pub description: String,
    /// Permissions granted by this role
    pub permissions: HashSet<Permission>,
}

impl Role {
    /// Create a new role with the given name and permissions.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            permissions: HashSet::new(),
        }
    }

    /// Add a permission to this role.
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self
    }

    /// Add multiple permissions to this role.
    pub fn with_permissions(mut self, permissions: impl IntoIterator<Item = Permission>) -> Self {
        self.permissions.extend(permissions);
        self
    }

    /// Check if this role has a specific permission.
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

/// Predefined roles for the system.
pub mod roles {
    use super::*;

    /// Admin role with full access.
    pub fn admin() -> Role {
        Role::new("admin", "Full system access").with_permissions(Permission::all().iter().copied())
    }

    /// Standard user role with basic access.
    pub fn user() -> Role {
        Role::new("user", "Standard user access").with_permissions([
            Permission::ProxyAccess,
            Permission::QuotaRead,
            Permission::UserRead, // Can read own user info
        ])
    }

    /// Read-only role for monitoring.
    pub fn readonly() -> Role {
        Role::new("readonly", "Read-only access").with_permissions([
            Permission::UserRead,
            Permission::QuotaRead,
            Permission::AuditRead,
        ])
    }

    /// API consumer role for external integrations.
    pub fn api_consumer() -> Role {
        Role::new("api_consumer", "API access only").with_permissions([Permission::ProxyAccess])
    }

    /// Get a role by name.
    pub fn get_by_name(name: &str) -> Option<Role> {
        match name {
            "admin" => Some(admin()),
            "user" => Some(user()),
            "readonly" => Some(readonly()),
            "api_consumer" => Some(api_consumer()),
            _ => None,
        }
    }

    /// List all predefined role names.
    pub fn all_names() -> &'static [&'static str] {
        &["admin", "user", "readonly", "api_consumer"]
    }
}

/// Check if a user with the given roles has a specific permission.
///
/// # Arguments
/// * `role_names` - Names of roles assigned to the user
/// * `required` - The permission to check for
///
/// # Returns
/// `true` if any of the user's roles grants the required permission
pub fn check_permission(role_names: &[String], required: Permission) -> bool {
    role_names.iter().any(|name| {
        roles::get_by_name(name)
            .map(|role| role.has_permission(required))
            .unwrap_or(false)
    })
}

/// Check if a user has all of the required permissions.
pub fn check_all_permissions(role_names: &[String], required: &[Permission]) -> bool {
    required.iter().all(|p| check_permission(role_names, *p))
}

/// Check if a user has any of the required permissions.
pub fn check_any_permission(role_names: &[String], required: &[Permission]) -> bool {
    required.iter().any(|p| check_permission(role_names, *p))
}

/// Get all permissions for a set of roles.
pub fn get_permissions(role_names: &[String]) -> HashSet<Permission> {
    let mut permissions = HashSet::new();
    for name in role_names {
        if let Some(role) = roles::get_by_name(name) {
            permissions.extend(role.permissions);
        }
    }
    permissions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_has_all_permissions() {
        let admin = roles::admin();
        for permission in Permission::all() {
            assert!(
                admin.has_permission(*permission),
                "Admin should have {:?}",
                permission
            );
        }
    }

    #[test]
    fn test_user_has_limited_permissions() {
        let user = roles::user();
        assert!(user.has_permission(Permission::ProxyAccess));
        assert!(user.has_permission(Permission::QuotaRead));
        assert!(!user.has_permission(Permission::AdminAccess));
        assert!(!user.has_permission(Permission::UserWrite));
    }

    #[test]
    fn test_readonly_cannot_write() {
        let readonly = roles::readonly();
        assert!(readonly.has_permission(Permission::UserRead));
        assert!(readonly.has_permission(Permission::QuotaRead));
        assert!(!readonly.has_permission(Permission::UserWrite));
        assert!(!readonly.has_permission(Permission::QuotaWrite));
        assert!(!readonly.has_permission(Permission::ProxyAccess));
    }

    #[test]
    fn test_check_permission_by_role_names() {
        let admin_roles = vec!["admin".to_string()];
        let user_roles = vec!["user".to_string()];
        let multi_roles = vec!["user".to_string(), "readonly".to_string()];

        assert!(check_permission(&admin_roles, Permission::AdminAccess));
        assert!(!check_permission(&user_roles, Permission::AdminAccess));
        assert!(check_permission(&user_roles, Permission::ProxyAccess));

        // Multi-role user gets combined permissions
        assert!(check_permission(&multi_roles, Permission::ProxyAccess)); // from user
        assert!(check_permission(&multi_roles, Permission::AuditRead)); // from readonly
    }

    #[test]
    fn test_get_permissions_combines_roles() {
        let roles = vec!["user".to_string(), "readonly".to_string()];
        let permissions = get_permissions(&roles);

        assert!(permissions.contains(&Permission::ProxyAccess));
        assert!(permissions.contains(&Permission::AuditRead));
        assert!(!permissions.contains(&Permission::AdminAccess));
    }

    #[test]
    fn test_check_all_permissions() {
        let admin_roles = vec!["admin".to_string()];
        let user_roles = vec!["user".to_string()];

        assert!(check_all_permissions(
            &admin_roles,
            &[Permission::UserRead, Permission::UserWrite]
        ));
        assert!(!check_all_permissions(
            &user_roles,
            &[Permission::UserRead, Permission::UserWrite]
        ));
    }

    #[test]
    fn test_check_any_permission() {
        let user_roles = vec!["user".to_string()];

        assert!(check_any_permission(
            &user_roles,
            &[Permission::AdminAccess, Permission::ProxyAccess]
        ));
        assert!(!check_any_permission(
            &user_roles,
            &[Permission::AdminAccess, Permission::ConfigWrite]
        ));
    }

    #[test]
    fn test_unknown_role_has_no_permissions() {
        let unknown_roles = vec!["unknown".to_string()];
        assert!(!check_permission(&unknown_roles, Permission::ProxyAccess));
        assert!(get_permissions(&unknown_roles).is_empty());
    }

    #[test]
    fn test_role_serialization() {
        let role = roles::admin();
        let json = serde_json::to_string(&role).unwrap();
        let parsed: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "admin");
        assert!(parsed.has_permission(Permission::AdminAccess));
    }

    #[test]
    fn test_permission_serialization() {
        let perm = Permission::AdminAccess;
        let json = serde_json::to_string(&perm).unwrap();
        assert_eq!(json, "\"admin_access\"");
        let parsed: Permission = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Permission::AdminAccess);
    }

    #[test]
    fn test_api_consumer_only_proxy() {
        let api = roles::api_consumer();
        assert!(api.has_permission(Permission::ProxyAccess));
        assert!(!api.has_permission(Permission::UserRead));
        assert!(!api.has_permission(Permission::QuotaRead));
    }

    #[test]
    fn test_get_role_by_name() {
        assert!(roles::get_by_name("admin").is_some());
        assert!(roles::get_by_name("user").is_some());
        assert!(roles::get_by_name("readonly").is_some());
        assert!(roles::get_by_name("api_consumer").is_some());
        assert!(roles::get_by_name("nonexistent").is_none());
    }
}
