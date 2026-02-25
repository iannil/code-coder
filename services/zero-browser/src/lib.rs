//! zero-browser - Browser automation with API learning capabilities.

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod browser;
pub mod error;
pub mod network;
pub mod pattern;
pub mod replay;
pub mod routes;

pub use error::BrowserError;
pub use pattern::types::{ApiPattern, AuthPattern, HeaderPattern};
