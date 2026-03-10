pub mod policy;
pub mod remote_policy;
pub mod secrets;
pub mod vault;

// Re-export pairing from zero_common
#[allow(unused_imports)]
pub use zero_core::common::security::pairing::PairingGuard;
pub use policy::{AutonomyLevel, SecurityPolicy};
pub use remote_policy::{RemotePolicy, RiskLevel, TaskContext};
#[allow(unused_imports)]
pub use secrets::SecretStore;
#[allow(unused_imports)]
pub use vault::{CredentialEntry, CredentialSummary, CredentialType, CredentialVault, LoginCredential, OAuthCredential};
