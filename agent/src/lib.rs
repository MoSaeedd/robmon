//! RobMon Agent Library
//!
//! A robot monitoring and management agent built with production Rust practices.
//! This library provides all the core functionality needed for the agent to operate reliably
//! in production environments.

pub mod api;
pub mod command;
pub mod config;
pub mod crypto;
pub mod error;
pub mod metrics;
pub mod models;
pub mod state;

pub use error::{AgentError, Result};
