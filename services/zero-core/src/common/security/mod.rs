//! Security primitives for Zero services.
//!
//! Provides secret storage, encryption, RBAC, pairing, and security utilities.

pub mod pairing;
pub mod rbac;
pub mod resource_rbac;
pub mod secrets;

pub use pairing::{constant_time_eq, is_public_bind, PairingGuard};
pub use rbac::{check_permission, Permission, Role};
pub use resource_rbac::{
    can_access_resource, Action, ResourcePermission, ResourceRole, ResourceRoleStore, ResourceType,
};
pub use secrets::SecretStore;

/// Autonomy level for agent operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutonomyLevel {
    /// Agent requires confirmation for all actions
    #[default]
    Supervised,
    /// Agent can perform safe operations automatically
    SemiAutonomous,
    /// Agent can perform all operations automatically
    Autonomous,
}

impl std::str::FromStr for AutonomyLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "supervised" => Ok(Self::Supervised),
            "semi-autonomous" | "semiautonomous" | "semi" => Ok(Self::SemiAutonomous),
            "autonomous" | "full" => Ok(Self::Autonomous),
            _ => Err(format!("Unknown autonomy level: {s}")),
        }
    }
}

/// Security policy for operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityPolicy {
    /// Autonomy level
    pub autonomy_level: AutonomyLevel,
    /// Only allow operations within workspace directory
    pub workspace_only: bool,
    /// Allowed shell commands (if semi-autonomous)
    pub allowed_commands: Vec<String>,
    /// Forbidden paths (always blocked)
    pub forbidden_paths: Vec<String>,
    /// Maximum actions per hour
    pub max_actions_per_hour: u32,
    /// Maximum cost per day in cents
    pub max_cost_per_day_cents: u32,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            autonomy_level: AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands: vec![
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "head".into(),
                "tail".into(),
            ],
            forbidden_paths: vec![
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "/etc/passwd".into(),
                "/etc/shadow".into(),
            ],
            max_actions_per_hour: 100,
            max_cost_per_day_cents: 1000, // $10
        }
    }
}

impl SecurityPolicy {
    /// Check if a command is allowed under this policy.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        match self.autonomy_level {
            AutonomyLevel::Supervised => false,
            AutonomyLevel::SemiAutonomous => {
                let cmd = command.split_whitespace().next().unwrap_or("");
                self.allowed_commands.iter().any(|c| c == cmd)
            }
            AutonomyLevel::Autonomous => true,
        }
    }

    /// Check if a path is forbidden under this policy.
    pub fn is_path_forbidden(&self, path: &str) -> bool {
        let expanded = shellexpand::tilde(path);
        self.forbidden_paths
            .iter()
            .any(|fp| expanded.starts_with(&*shellexpand::tilde(fp)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomy_level_parse() {
        assert_eq!(
            "supervised".parse::<AutonomyLevel>().unwrap(),
            AutonomyLevel::Supervised
        );
        assert_eq!(
            "autonomous".parse::<AutonomyLevel>().unwrap(),
            AutonomyLevel::Autonomous
        );
        assert_eq!(
            "semi-autonomous".parse::<AutonomyLevel>().unwrap(),
            AutonomyLevel::SemiAutonomous
        );
    }

    #[test]
    fn test_security_policy_command_check() {
        let policy = SecurityPolicy::default();
        assert!(!policy.is_command_allowed("ls")); // supervised = all blocked

        let semi_policy = SecurityPolicy {
            autonomy_level: AutonomyLevel::SemiAutonomous,
            ..Default::default()
        };
        assert!(semi_policy.is_command_allowed("ls"));
        assert!(!semi_policy.is_command_allowed("rm"));
    }

    #[test]
    fn test_security_policy_path_check() {
        let policy = SecurityPolicy::default();
        assert!(policy.is_path_forbidden("~/.ssh/id_rsa"));
        assert!(!policy.is_path_forbidden("/tmp/test.txt"));
    }
}
