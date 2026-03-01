//! Configuration management for Zero services.
//!
//! This module is organized into sub-modules for better maintainability.
//! All types are re-exported at this level for backward compatibility.

// Sub-modules (extracted for better organization)
pub mod llm;
pub mod network;
pub mod secrets;
pub mod services;
pub mod timeout;

// Main types module (contains remaining types)
mod types;

// Re-export everything from types for backward compatibility
pub use types::*;

// Re-export sub-module types (these will gradually replace types in types.rs)
// Note: Currently types.rs still contains these types. Once verified,
// remove duplicates from types.rs and use these re-exports instead.

// For now, we use the types from types.rs to maintain backward compatibility.
// The sub-modules are ready for future migration.
