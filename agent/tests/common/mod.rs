//! Shared test utilities for the robmon-agent integration test suite.
//!
//! This module provides helpers used across multiple test files to reduce
//! duplication and ensure consistent test setup.

use robmon_agent::models::{AgentState, AuthState, LoadAverage, MeshState, RobotMetadata, SystemMetrics};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// Seed for test-specific temporary directories to avoid collisions between
/// tests running in parallel.
const TEST_PREFIX: &str = "robmon_agent_test";

/// Create an `AgentState` with fully deterministic data for testing.
pub fn test_agent_state(robot_id: &str) -> AgentState {
    AgentState {
        metadata: RobotMetadata {
            robot_id: robot_id.to_string(),
            hostname: "test-host".into(),
            os: "test-os".into(),
            arch: "x86_64".into(),
            ros_version: "humble".into(),
            agent_version: "0.1.0".into(),
        },
        metrics: SystemMetrics {
            cpu_usage: 15.0,
            cpu_cores: 4,
            memory_total_bytes: 8_000_000_000,
            memory_used_bytes: 2_000_000_000,
            memory_available_bytes: 6_000_000_000,
            load_average: LoadAverage {
                one: 0.5,
                five: 0.4,
                fifteen: 0.3,
            },
        },
        last_seen: DateTime::parse_from_rfc3339("2025-05-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        command_history: vec![],
        mesh: MeshState::default(),
    }
}

/// Create a temporary directory unique to the calling test.
///
/// The directory is created automatically. Callers should clean it up
/// when done (use `scoped_temp_dir` for automatic cleanup).
pub fn unique_temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{}_{}_{}", TEST_PREFIX, label, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Run a closure with a uniquely-named temporary directory that is
/// automatically cleaned up after the closure returns.
pub fn with_temp_dir<F>(label: &str, f: F)
where
    F: FnOnce(&PathBuf),
{
    let dir = unique_temp_dir(label);
    f(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// Persist an `AuthState` to disk at the given path (simulates
/// `StateManager::save_auth_token`).
pub fn write_auth_token(path: &PathBuf, token: &str) {
    let auth = AuthState { token: token.into() };
    let contents = serde_json::to_string_pretty(&auth).unwrap();
    fs::write(path, contents).unwrap();
}

/// Load an `AuthState` from disk at the given path (simulates
/// `StateManager::load_auth_token`).
pub fn read_auth_token(path: &PathBuf) -> AuthState {
    let raw = fs::read_to_string(path).unwrap();
    serde_json::from_str(&raw).unwrap()
}

/// Persist an `AgentState` to disk at the given path.
pub fn write_agent_state(path: &PathBuf, state: &AgentState) {
    let contents = serde_json::to_string_pretty(state).unwrap();
    fs::write(path, contents).unwrap();
}

/// Load an `AgentState` from disk at the given path.
pub fn read_agent_state(path: &PathBuf) -> AgentState {
    let raw = fs::read_to_string(path).unwrap();
    serde_json::from_str(&raw).unwrap()
}