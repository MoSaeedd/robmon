//! Integration tests for the command execution module.
//!
//! These tests execute real shell commands via `execute_command` and
//! verify stdout capture, stderr capture, error handling, and edge cases.
//! All tests use the tokio runtime.

use robmon_agent::command::execute_command;

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn echo_command() {
    let result = execute_command("echo 'hello world'").await;
    assert!(
        result.contains("hello world"),
        "Expected output to contain 'hello world', got: {result}"
    );
}

#[tokio::test]
async fn multiple_commands() {
    let result = execute_command("echo first && echo second && echo third").await;
    assert!(result.contains("first"));
    assert!(result.contains("second"));
    assert!(result.contains("third"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failing_command_returns_error_prefix() {
    let result = execute_command("exit 1").await;
    assert!(
        result.starts_with("error:"),
        "Expected error prefix, got: {result}"
    );
}

#[tokio::test]
async fn stderr_captured_in_error_output() {
    let result = execute_command("echo 'stdout msg' && echo 'stderr msg' >&2 && exit 1").await;
    assert!(result.starts_with("error:"), "Expected error prefix");
    assert!(
        result.contains("stderr msg"),
        "stderr should appear in error output"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_command_succeeds() {
    let result = execute_command("").await;
    assert!(
        !result.starts_with("error:"),
        "Empty command should not error"
    );
}

#[tokio::test]
async fn multi_line_output() {
    let result = execute_command("printf 'line1\nline2\n'").await;
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
}