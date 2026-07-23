//! Tests for the CLI configuration parser (`Config` struct).
//!
//! Uses clap's `parse_from` to simulate command-line argument parsing
//! without needing a real binary invocation. This style is the standard
//! approach for testing clap-based configs in production Rust projects.

use clap::Parser;
use robmon_agent::config::Config;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Default values
// ---------------------------------------------------------------------------

#[test]
fn defaults_are_sane() {
    let config = Config::parse_from(&["robmon-agent"]);
    assert_eq!(config.control_plane_url, "http://127.0.0.1:8080");
    assert_eq!(config.mesh_public_port, 51820);
    assert_eq!(config.service_protocol, "http");
    assert_eq!(config.heartbeat_interval, 10);
    assert_eq!(config.metrics_interval, 1);
    assert!(config.mesh_public_ip.is_none());
    assert!(config.control_plane_user.is_none());
    assert!(config.control_plane_password.is_none());
    assert!(config.service_name.is_none());
    assert!(config.service_port.is_none());
    assert!(config.service_host.is_none());
    assert!(!config.logout);
    assert!(!config.login);
}

// ---------------------------------------------------------------------------
// Control plane URL
// ---------------------------------------------------------------------------

#[test]
fn custom_control_plane_url() {
    let config = Config::parse_from(&[
        "robmon-agent",
        "--control-plane-url",
        "https://control.example.com:9090",
    ]);
    assert_eq!(config.control_plane_url, "https://control.example.com:9090");
}

// ---------------------------------------------------------------------------
// Mesh networking
// ---------------------------------------------------------------------------

#[test]
fn custom_mesh_public_ip() {
    let config = Config::parse_from(&[
        "robmon-agent",
        "--mesh-public-ip",
        "203.0.113.50",
    ]);
    assert_eq!(config.mesh_public_ip.unwrap(), "203.0.113.50");
}

#[test]
fn custom_mesh_public_port() {
    let config = Config::parse_from(&["robmon-agent", "--mesh-public-port", "51821"]);
    assert_eq!(config.mesh_public_port, 51821);
}

// ---------------------------------------------------------------------------
// Service registration
// ---------------------------------------------------------------------------

#[test]
fn service_registration_settings() {
    let config = Config::parse_from(&[
        "robmon-agent",
        "--service-name",
        "my-robot",
        "--service-port",
        "3000",
        "--service-host",
        "10.0.0.5",
        "--service-protocol",
        "https",
    ]);
    assert_eq!(config.service_name.unwrap(), "my-robot");
    assert_eq!(config.service_port.unwrap(), 3000);
    assert_eq!(config.service_host.unwrap(), "10.0.0.5");
    assert_eq!(config.service_protocol, "https");
}

// ---------------------------------------------------------------------------
// Timing intervals
// ---------------------------------------------------------------------------

#[test]
fn custom_heartbeat_interval() {
    let config = Config::parse_from(&["robmon-agent", "--heartbeat-interval", "30"]);
    assert_eq!(config.heartbeat_interval, 30);
    assert_eq!(config.heartbeat_duration(), Duration::from_secs(30));
}

#[test]
fn custom_metrics_interval() {
    let config = Config::parse_from(&["robmon-agent", "--metrics-interval", "5"]);
    assert_eq!(config.metrics_interval, 5);
    assert_eq!(config.metrics_duration(), Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// Boolean flags
// ---------------------------------------------------------------------------

#[test]
fn login_and_logout_flags() {
    let config = Config::parse_from(&["robmon-agent", "--login"]);
    assert!(config.login);
    assert!(!config.logout);

    let config = Config::parse_from(&["robmon-agent", "--logout"]);
    assert!(config.logout);
    assert!(!config.login);
}