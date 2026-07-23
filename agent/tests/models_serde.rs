//! Serialization round-trip tests for all RobMon agent models.
//!
//! These tests verify that every model struct correctly serializes to and
//! deserializes from JSON, including camelCase field renames, default values,
//! and optional fields.

use robmon_agent::models::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// RobotMetadata
// ---------------------------------------------------------------------------

#[test]
fn robot_metadata_round_trip() {
    let meta = RobotMetadata {
        robot_id: "robot-abc-123".into(),
        hostname: "test-robot".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        ros_version: "humble".into(),
        agent_version: "0.1.0".into(),
    };

    let json = serde_json::to_string_pretty(&meta).unwrap();
    let deserialized: RobotMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.robot_id, "robot-abc-123");
    assert_eq!(deserialized.hostname, "test-robot");
    assert_eq!(deserialized.os, "linux");
    assert_eq!(deserialized.arch, "x86_64");
    assert_eq!(deserialized.ros_version, "humble");
    assert_eq!(deserialized.agent_version, "0.1.0");
}

#[test]
fn robot_metadata_default_has_unique_id() {
    let meta = RobotMetadata::default();
    assert!(!meta.robot_id.is_empty());
    assert!(meta.robot_id.contains('-'), "robot_id should contain a UUID separator");
    assert_eq!(meta.os, std::env::consts::OS);
    assert_eq!(meta.arch, std::env::consts::ARCH);
    assert_eq!(meta.agent_version, "0.1.0");
}

// ---------------------------------------------------------------------------
// SystemMetrics & LoadAverage
// ---------------------------------------------------------------------------

#[test]
fn system_metrics_round_trip() {
    let metrics = SystemMetrics {
        cpu_usage: 42.5,
        cpu_cores: 8,
        memory_total_bytes: 16_000_000_000,
        memory_used_bytes: 8_000_000_000,
        memory_available_bytes: 8_000_000_000,
        load_average: LoadAverage {
            one: 1.5,
            five: 1.2,
            fifteen: 1.0,
        },
    };

    let json = serde_json::to_string_pretty(&metrics).unwrap();
    let deserialized: SystemMetrics = serde_json::from_str(&json).unwrap();

    assert!((deserialized.cpu_usage - 42.5).abs() < f32::EPSILON);
    assert_eq!(deserialized.cpu_cores, 8);
    assert_eq!(deserialized.memory_total_bytes, 16_000_000_000);
    assert_eq!(deserialized.memory_used_bytes, 8_000_000_000);
    assert_eq!(deserialized.load_average.one, 1.5);
    assert_eq!(deserialized.load_average.five, 1.2);
    assert_eq!(deserialized.load_average.fifteen, 1.0);
}

// ---------------------------------------------------------------------------
// AgentState
// ---------------------------------------------------------------------------

#[test]
fn agent_state_round_trip() {
    let state = AgentState {
        metadata: RobotMetadata {
            robot_id: "robot-1".into(),
            hostname: "robot-1".into(),
            os: "linux".into(),
            arch: "aarch64".into(),
            ros_version: "iron".into(),
            agent_version: "0.1.0".into(),
        },
        metrics: SystemMetrics {
            cpu_usage: 10.0,
            cpu_cores: 4,
            memory_total_bytes: 8_000_000_000,
            memory_used_bytes: 4_000_000_000,
            memory_available_bytes: 4_000_000_000,
            load_average: LoadAverage {
                one: 2.0,
                five: 1.8,
                fifteen: 1.5,
            },
        },
        last_seen: chrono::DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        command_history: vec!["echo hello => hello".into()],
        mesh: MeshState {
            mesh_ip: Some("10.42.0.2".into()),
            public_ip: Some("203.0.113.10".into()),
            public_port: Some(51820),
            subnet: Some("10.42.0.0/16".into()),
            gateway: Some("10.42.0.1".into()),
            peers: vec![MeshPeer {
                node_id: "robot-b".into(),
                hostname: "robot-b".into(),
                mesh_ip: "10.42.0.1".into(),
                public_ip: "203.0.113.11".into(),
                public_port: 51821,
                online: true,
            }],
        },
    };

    let json = serde_json::to_string_pretty(&state).unwrap();
    let deserialized: AgentState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.metadata.robot_id, "robot-1");
    assert_eq!(deserialized.command_history.len(), 1);
    assert_eq!(deserialized.command_history[0], "echo hello => hello");
    assert_eq!(deserialized.mesh.mesh_ip.unwrap(), "10.42.0.2");
    assert_eq!(deserialized.mesh.peers.len(), 1);
    assert_eq!(deserialized.mesh.peers[0].node_id, "robot-b");
}

#[test]
fn agent_state_default_is_valid() {
    let state = AgentState::default();
    assert!(!state.metadata.robot_id.is_empty());
    assert!(!state.metadata.hostname.is_empty());
    assert_eq!(state.metadata.agent_version, "0.1.0");
    assert!(state.metrics.cpu_cores >= 1);
    assert!(state.metrics.memory_total_bytes > 0);
    assert!(state.command_history.is_empty());
    assert!(state.mesh.mesh_ip.is_none());
}

// ---------------------------------------------------------------------------
// CamelCase field mappings (API contract compliance)
// ---------------------------------------------------------------------------

#[test]
fn login_response_access_token_is_camel_case() {
    // Deserialize from camelCase (API response format)
    let json = r#"{"accessToken":"my-token"}"#;
    let response: LoginResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.access_token, "my-token");

    // Serialize produces camelCase
    let response = LoginResponse {
        access_token: "my-token".into(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("accessToken"), "JSON should use camelCase: {json}");
    assert!(!json.contains("access_token"), "JSON should NOT use snake_case: {json}");
}

#[test]
fn service_registration_uses_camel_case() {
    let json = r#"{"serviceName":"svc","host":"0.0.0.0","port":3000,"protocol":"grpc","meta":{"key":"val"}}"#;
    let reg: ServiceRegistration = serde_json::from_str(json).unwrap();
    assert_eq!(reg.service_name, "svc");
    assert_eq!(reg.port, 3000);
    assert_eq!(reg.protocol, "grpc");
    assert_eq!(reg.meta.get("key").unwrap(), "val");
}

// ---------------------------------------------------------------------------
// Auth & Authentication models
// ---------------------------------------------------------------------------

#[test]
fn auth_state_round_trip() {
    let auth = AuthState {
        token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".into(),
    };
    let json = serde_json::to_string(&auth).unwrap();
    let deserialized: AuthState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
}

#[test]
fn login_payload_round_trip() {
    let payload = LoginPayload {
        username: "admin".into(),
        password: "secret".into(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: LoginPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.username, "admin");
    assert_eq!(deserialized.password, "secret");
}

// ---------------------------------------------------------------------------
// CommandResponse
// ---------------------------------------------------------------------------

#[test]
fn command_response_round_trip() {
    let resp = CommandResponse {
        commands: vec!["ls -la".into(), "df -h".into()],
    };
    let json = serde_json::to_string(&resp).unwrap();
    let deserialized: CommandResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.commands.len(), 2);
    assert_eq!(deserialized.commands[1], "df -h");
}

#[test]
fn command_response_empty() {
    let resp = CommandResponse { commands: vec![] };
    let json = serde_json::to_string(&resp).unwrap();
    assert_eq!(json, r#"{"commands":[]}"#);
}

// ---------------------------------------------------------------------------
// ServiceRegistration
// ---------------------------------------------------------------------------

#[test]
fn service_registration_empty_meta() {
    let reg = ServiceRegistration {
        service_name: "test-svc".into(),
        host: "127.0.0.1".into(),
        port: 8080,
        protocol: "http".into(),
        meta: HashMap::new(),
    };
    assert!(reg.meta.is_empty());
}

// ---------------------------------------------------------------------------
// MeshState
// ---------------------------------------------------------------------------

#[test]
fn mesh_state_default_is_empty() {
    let state = MeshState::default();
    assert!(state.mesh_ip.is_none());
    assert!(state.public_ip.is_none());
    assert!(state.public_port.is_none());
    assert!(state.subnet.is_none());
    assert!(state.gateway.is_none());
    assert!(state.peers.is_empty());
}

#[test]
fn mesh_state_accepts_peers() {
    let mut state = MeshState::default();
    assert!(state.peers.is_empty());

    state.peers.push(MeshPeer {
        node_id: "peer-1".into(),
        hostname: "peer-1".into(),
        mesh_ip: "10.42.0.2".into(),
        public_ip: "203.0.113.2".into(),
        public_port: 51821,
        online: true,
    });
    assert_eq!(state.peers.len(), 1);
}

#[test]
fn mesh_peer_direct_construction() {
    // MeshPeer does not implement Default; verify we can construct it
    let peer = MeshPeer {
        node_id: "test-node".into(),
        hostname: "test-host".into(),
        mesh_ip: "10.42.0.1".into(),
        public_ip: "203.0.113.1".into(),
        public_port: 51820,
        online: true,
    };
    assert_eq!(peer.node_id, "test-node");
    assert!(peer.online);
}

// ---------------------------------------------------------------------------
// Mesh Payloads & Responses
// ---------------------------------------------------------------------------

#[test]
fn mesh_join_payload_round_trip() {
    let payload = MeshJoinPayload {
        node_id: "node-42".into(),
        hostname: "node-42".into(),
        public_ip: "10.0.0.1".into(),
        public_port: 51820,
    };
    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: MeshJoinPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.node_id, "node-42");
    assert_eq!(deserialized.public_ip, "10.0.0.1");
    assert_eq!(deserialized.public_port, 51820);
}

#[test]
fn mesh_join_response_round_trip() {
    let response = MeshJoinResponse {
        mesh_ip: "10.42.0.5".into(),
        subnet: "10.42.0.0/16".into(),
        gateway: "10.42.0.1".into(),
        peers: vec![MeshPeer {
            node_id: "robot-a".into(),
            hostname: "robot-a".into(),
            mesh_ip: "10.42.0.2".into(),
            public_ip: "203.0.113.10".into(),
            public_port: 51820,
            online: true,
        }],
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: MeshJoinResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.mesh_ip, "10.42.0.5");
    assert_eq!(deserialized.peers.len(), 1);
    assert_eq!(deserialized.peers[0].public_ip, "203.0.113.10");
}

#[test]
fn mesh_heartbeat_payload_round_trip() {
    let payload = MeshHeartbeatPayload {
        node_id: "node-99".into(),
        public_ip: "10.0.0.99".into(),
        public_port: 51822,
    };
    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: MeshHeartbeatPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.node_id, "node-99");
    assert_eq!(deserialized.public_port, 51822);
}

#[test]
fn mesh_heartbeat_response_round_trip() {
    let response = MeshHeartbeatResponse {
        mesh_ip: "10.42.0.3".into(),
        peers: vec![],
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: MeshHeartbeatResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.mesh_ip, "10.42.0.3");
    assert!(deserialized.peers.is_empty());
}

#[test]
fn mesh_heartbeat_response_with_peers() {
    let response = MeshHeartbeatResponse {
        mesh_ip: "10.42.0.3".into(),
        peers: vec![MeshPeer {
            node_id: "robot-a".into(),
            hostname: "robot-a".into(),
            mesh_ip: "10.42.0.2".into(),
            public_ip: "203.0.113.10".into(),
            public_port: 51820,
            online: true,
        }],
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: MeshHeartbeatResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.mesh_ip, "10.42.0.3");
    assert_eq!(deserialized.peers.len(), 1);
}

// ---------------------------------------------------------------------------
// Agent version constant
// ---------------------------------------------------------------------------

#[test]
fn agent_version_matches_cargo_toml() {
    assert_eq!(AGENT_VERSION, "0.1.0");
}