//! Tests for AgentError display formatting and trait implementations.
//!
//! The error module defines a thiserror-derived enum with Display,
//! From<io::Error>, and From<serde_json::Error> impls. These tests
//! verify the error messages match expected API contracts.

use robmon_agent::error::AgentError;

// ---------------------------------------------------------------------------
// Display formatting for each variant
// ---------------------------------------------------------------------------

#[test]
fn config_error_message() {
    let err = AgentError::ConfigError("missing field 'port'".into());
    assert_eq!(format!("{}", err), "Configuration error: missing field 'port'");
}

#[test]
fn network_error_message() {
    let err = AgentError::NetworkError("connection refused".into());
    assert_eq!(format!("{}", err), "Network error: connection refused");
}

#[test]
fn io_error_message() {
    use std::io::ErrorKind;
    let io_err = std::io::Error::new(ErrorKind::NotFound, "file not found");
    let err = AgentError::IoError(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("IO error"));
    assert!(msg.contains("file not found"));
}

#[test]
fn serialization_error_message() {
    let serde_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
    let err = AgentError::SerializationError(serde_err);
    let msg = format!("{}", err);
    assert!(msg.contains("Serialization error"));
}

#[test]
fn authentication_error_message() {
    let err = AgentError::AuthenticationFailed("token expired".into());
    assert_eq!(format!("{}", err), "Authentication failed: token expired");
}

#[test]
fn mesh_error_message() {
    let err = AgentError::MeshError("peer unreachable".into());
    assert_eq!(format!("{}", err), "Mesh network error: peer unreachable");
}

#[test]
fn state_error_message() {
    let err = AgentError::StateError("corrupted database".into());
    assert_eq!(format!("{}", err), "State management error: corrupted database");
}

#[test]
fn command_execution_error_message() {
    let err = AgentError::CommandExecutionError("segmentation fault".into());
    assert_eq!(
        format!("{}", err),
        "Command execution failed: segmentation fault"
    );
}

#[test]
fn metrics_error_message() {
    let err = AgentError::MetricsError("permission denied".into());
    assert_eq!(
        format!("{}", err),
        "System metrics collection failed: permission denied"
    );
}

// ---------------------------------------------------------------------------
// From trait implementations (auto-conversions)
// ---------------------------------------------------------------------------

#[test]
fn from_io_error() {
    use std::io::ErrorKind;
    let io_err = std::io::Error::new(ErrorKind::PermissionDenied, "access denied");
    let err: AgentError = io_err.into();
    assert!(format!("{}", err).contains("IO error"));
}

#[test]
fn from_serde_json_error() {
    let serde_err = serde_json::from_str::<serde_json::Value>("{bad json}").unwrap_err();
    let err: AgentError = serde_err.into();
    assert!(format!("{}", err).contains("Serialization error"));
}

// ---------------------------------------------------------------------------
// Result type alias
// ---------------------------------------------------------------------------

#[test]
fn result_ok_value() {
    let ok: robmon_agent::error::Result<i32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);
}

#[test]
fn result_err_value() {
    let err: robmon_agent::error::Result<i32> =
        Err(AgentError::ConfigError("oops".into()));
    assert!(err.is_err());
}