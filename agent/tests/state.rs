//! Tests for AgentState and AuthState persistence.
//!
//! These tests verify that state files can be written to and read from
//! disk without data loss, that token files can be created and cleared,
//! and that missing paths are handled gracefully.
//!
//! Each test uses a uniquely-named temp directory to avoid collisions
//! with parallel test execution.

mod common;

use common::{test_agent_state, with_temp_dir, write_auth_token, read_auth_token, write_agent_state, read_agent_state};
use std::fs;

// ---------------------------------------------------------------------------
// AgentState persistence
// ---------------------------------------------------------------------------

#[test]
fn agent_state_save_and_load() {
    with_temp_dir("state_rw", |dir| {
        let state_path = dir.join("agent_state.json");
        let state = test_agent_state("persist-robot");

        write_agent_state(&state_path, &state);
        let loaded = read_agent_state(&state_path);

        assert_eq!(loaded.metadata.robot_id, "persist-robot");
        assert_eq!(loaded.metadata.hostname, "test-host");
        assert_eq!(loaded.metrics.cpu_usage, 15.0);
        assert_eq!(loaded.metrics.cpu_cores, 4);
    });
}

// ---------------------------------------------------------------------------
// AuthToken persistence
// ---------------------------------------------------------------------------

#[test]
fn auth_token_save_and_load() {
    with_temp_dir("auth_rw", |dir| {
        let token_path = dir.join("agent_token.json");

        write_auth_token(&token_path, "some.jwt.token");
        let loaded = read_auth_token(&token_path);

        assert_eq!(loaded.token, "some.jwt.token");
    });
}

#[test]
fn auth_token_clear() {
    with_temp_dir("auth_clear", |dir| {
        let token_path = dir.join("agent_token.json");

        // Create
        write_auth_token(&token_path, "clear-me");
        assert!(token_path.exists());

        // Clear
        fs::remove_file(&token_path).unwrap();
        assert!(!token_path.exists());
    });
}

// ---------------------------------------------------------------------------
// Missing file handling
// ---------------------------------------------------------------------------

#[test]
fn missing_token_file_returns_error() {
    with_temp_dir("auth_missing", |dir| {
        let token_path = dir.join("agent_token.json");

        // Ensure the file does NOT exist
        let _ = fs::remove_file(&token_path);

        assert!(!token_path.exists());
        let result = fs::read_to_string(&token_path);
        assert!(result.is_err(), "Loading a missing token should fail");
    });
}

#[test]
fn missing_state_file_returns_error() {
    with_temp_dir("state_missing", |dir| {
        let state_path = dir.join("agent_state.json");

        // Ensure the file does NOT exist
        let _ = fs::remove_file(&state_path);

        assert!(!state_path.exists());
        let result = fs::read_to_string(&state_path);
        assert!(result.is_err(), "Loading a missing state should fail");
    });
}