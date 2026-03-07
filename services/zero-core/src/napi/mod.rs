//! NAPI bindings for Node.js integration
//!
//! This module provides FFI bindings to expose zero-core functionality
//! to JavaScript/TypeScript via napi-rs.

#[cfg(feature = "napi-bindings")]
mod audit;

#[cfg(feature = "napi-bindings")]
mod autonomous;

#[cfg(feature = "napi-bindings")]
mod bindings;

#[cfg(feature = "napi-bindings")]
mod config;

#[cfg(feature = "napi-bindings")]
mod context;

#[cfg(feature = "napi-bindings")]
mod embedding;

#[cfg(feature = "napi-bindings")]
mod git;

#[cfg(feature = "napi-bindings")]
mod graph;

#[cfg(feature = "napi-bindings")]
mod history;

#[cfg(feature = "napi-bindings")]
mod hook;

#[cfg(feature = "napi-bindings")]
mod ignore;

#[cfg(feature = "napi-bindings")]
mod index;

#[cfg(feature = "napi-bindings")]
mod java;

#[cfg(feature = "napi-bindings")]
mod keyring;

#[cfg(feature = "napi-bindings")]
mod markdown;

#[cfg(feature = "napi-bindings")]
mod memory;

#[cfg(feature = "napi-bindings")]
mod observability;

#[cfg(feature = "napi-bindings")]
mod protocol;

#[cfg(feature = "napi-bindings")]
mod provider;

#[cfg(feature = "napi-bindings")]
mod pty;

#[cfg(feature = "napi-bindings")]
mod schema;

#[cfg(feature = "napi-bindings")]
mod security;

#[cfg(feature = "napi-bindings")]
mod shell_parser;

#[cfg(feature = "napi-bindings")]
mod skill;

#[cfg(feature = "napi-bindings")]
mod storage;

#[cfg(feature = "napi-bindings")]
mod tools;

#[cfg(feature = "napi-bindings")]
mod tool_registry;

#[cfg(feature = "napi-bindings")]
mod trace;

#[cfg(feature = "napi-bindings")]
mod watcher;

#[cfg(feature = "napi-bindings")]
mod web;

#[cfg(feature = "napi-bindings")]
pub use audit::*;

#[cfg(feature = "napi-bindings")]
pub use autonomous::*;

#[cfg(feature = "napi-bindings")]
pub use bindings::*;

#[cfg(feature = "napi-bindings")]
pub use config::*;

#[cfg(feature = "napi-bindings")]
pub use context::*;

#[cfg(feature = "napi-bindings")]
pub use embedding::*;

#[cfg(feature = "napi-bindings")]
pub use git::*;

#[cfg(feature = "napi-bindings")]
pub use graph::*;

#[cfg(feature = "napi-bindings")]
pub use history::*;

#[cfg(feature = "napi-bindings")]
pub use hook::*;

#[cfg(feature = "napi-bindings")]
pub use ignore::*;

#[cfg(feature = "napi-bindings")]
pub use index::*;

#[cfg(feature = "napi-bindings")]
pub use java::*;

#[cfg(feature = "napi-bindings")]
pub use keyring::*;

#[cfg(feature = "napi-bindings")]
pub use markdown::*;

#[cfg(feature = "napi-bindings")]
pub use memory::*;

#[cfg(feature = "napi-bindings")]
pub use observability::*;

#[cfg(feature = "napi-bindings")]
pub use protocol::*;

#[cfg(feature = "napi-bindings")]
pub use provider::*;

#[cfg(feature = "napi-bindings")]
pub use pty::*;

#[cfg(feature = "napi-bindings")]
pub use schema::*;

#[cfg(feature = "napi-bindings")]
pub use security::*;

#[cfg(feature = "napi-bindings")]
pub use shell_parser::*;

#[cfg(feature = "napi-bindings")]
pub use skill::*;

#[cfg(feature = "napi-bindings")]
pub use storage::*;

#[cfg(feature = "napi-bindings")]
pub use tools::*;

#[cfg(feature = "napi-bindings")]
pub use tool_registry::*;

#[cfg(feature = "napi-bindings")]
pub use trace::*;

#[cfg(feature = "napi-bindings")]
pub use watcher::*;

#[cfg(feature = "napi-bindings")]
pub use web::*;
