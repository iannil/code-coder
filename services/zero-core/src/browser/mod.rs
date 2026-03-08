//! Browser automation with API learning capabilities.
//!
//! Originally from `zero-browser` crate, now merged into `zero-core`.

pub mod browser;
pub mod error;
pub mod network;
pub mod pattern;
pub mod replay;
pub mod routes;

pub use browser::{LearnFilter, Session, SessionConfig, SessionManager};
pub use error::BrowserError;
pub use network::NetworkMonitor;
pub use pattern::{extract_patterns, ApiPattern, AuthPattern, HeaderPattern};
pub use replay::{ReplayExecutor, ReplayParams, ReplayResponse};
pub use routes::{build_router, AppState as BrowserAppState};
