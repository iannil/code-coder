pub mod policy;
pub mod secrets;
pub mod vault;

// Re-export pairing from zero_common
pub use zero_common::security::pairing;
#[allow(unused_imports)]
pub use zero_common::security::pairing::PairingGuard;
pub use policy::{AutonomyLevel, SecurityPolicy};
#[allow(unused_imports)]
pub use secrets::SecretStore;
#[allow(unused_imports)]
pub use vault::{CredentialEntry, CredentialSummary, CredentialType, CredentialVault, LoginCredential, OAuthCredential};
