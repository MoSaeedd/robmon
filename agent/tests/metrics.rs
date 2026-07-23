//! Tests for the MetricsCollector and standalone metrics helper.
//!
//! These tests execute real system metric collection via sysinfo,
//! verifying that returned values are within plausible ranges and
//! consistent across consecutive calls.

use robmon_agent::metrics::{MetricsCollector, collect_system_metrics};
use sysinfo::System;

// ---------------------------------------------------------------------------
// MetricsCollector lifecycle
// ---------------------------------------------------------------------------

#[test]
fn collector_collects_with_at_least_one_cpu() {
    let mut collector = MetricsCollector::new();
    let metrics = collector.collect().expect("Metrics collection should succeed");
    assert!(
        metrics.cpu_cores >= 1,
        "Expected at least 1 CPU on the host system"
    );
}

// ---------------------------------------------------------------------------
// Plausibility checks
// ---------------------------------------------------------------------------

#[test]
fn collected_metrics_are_plausible() {
    let mut collector = MetricsCollector::new();
    let metrics = collector.collect().expect("Metrics collection should succeed");

    // CPU usage: 0–100%
    assert!(metrics.cpu_usage >= 0.0, "CPU usage should be >= 0");
    assert!(metrics.cpu_usage <= 100.0, "CPU usage should be <= 100");

    // At least 1 core
    assert!(metrics.cpu_cores >= 1, "Expected at least 1 CPU core");

    // Total memory must be positive
    assert!(metrics.memory_total_bytes > 0, "Total memory should be > 0");

    // Available memory (if reported) must not exceed total
    if metrics.memory_available_bytes > 0 {
        assert!(
            metrics.memory_available_bytes <= metrics.memory_total_bytes,
            "Available memory should not exceed total memory"
        );
    }
}

#[test]
fn load_average_values_are_finite() {
    let mut collector = MetricsCollector::new();
    let metrics = collector.collect().expect("Metrics collection should succeed");

    assert!(metrics.load_average.one.is_finite(), "Load avg 1min should be finite");
    assert!(metrics.load_average.five.is_finite(), "Load avg 5min should be finite");
    assert!(metrics.load_average.fifteen.is_finite(), "Load avg 15min should be finite");
}

// ---------------------------------------------------------------------------
// Stability across consecutive calls
// ---------------------------------------------------------------------------

#[test]
fn consecutive_collections_are_consistent() {
    let mut collector = MetricsCollector::new();
    let first = collector.collect().expect("First collection failed");
    let second = collector.collect().expect("Second collection failed");

    assert_eq!(first.cpu_cores, second.cpu_cores, "CPU cores should be stable");
    assert_eq!(
        first.memory_total_bytes,
        second.memory_total_bytes,
        "Total memory should be stable"
    );
}

// ---------------------------------------------------------------------------
// Standalone function
// ---------------------------------------------------------------------------

#[test]
fn standalone_collect_system_metrics() {
    let mut system = System::new_all();
    let metrics = collect_system_metrics(&mut system);

    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.cpu_cores >= 1);
    assert!(metrics.memory_total_bytes > 0);
}