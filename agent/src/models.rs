use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RobotMetadata {
    pub robot_id: String,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub ros_version: String,
    pub agent_version: String,
}

impl Default for RobotMetadata {
    fn default() -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown-host".to_string());

        Self {
            robot_id: format!("{}-{}", hostname, Uuid::new_v4()),
            hostname,
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            ros_version: "unknown".to_string(),
            agent_version: AGENT_VERSION.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_available_bytes: u64,
    pub load_average: LoadAverage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeshPeer {
    pub node_id: String,
    pub hostname: String,
    pub mesh_ip: String,
    pub public_ip: String,
    pub public_port: u16,
    pub online: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MeshState {
    pub mesh_ip: Option<String>,
    pub public_ip: Option<String>,
    pub public_port: Option<u16>,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub peers: Vec<MeshPeer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentState {
    pub metadata: RobotMetadata,
    pub metrics: SystemMetrics,
    pub last_seen: DateTime<Utc>,
    pub command_history: Vec<String>,
    #[serde(default)]
    pub mesh: MeshState,
}

impl Default for AgentState {
    fn default() -> Self {
        let metadata = RobotMetadata::default();
        let mut system = sysinfo::System::new_all();
        let metrics = crate::metrics::collect_system_metrics(&mut system);

        Self {
            metadata,
            metrics,
            last_seen: Utc::now(),
            command_history: Vec::new(),
            mesh: MeshState::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandResponse {
    pub commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthState {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceRegistration {
    #[serde(rename = "serviceName")]
    pub service_name: String,
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub meta: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshJoinPayload {
    pub node_id: String,
    pub hostname: String,
    pub public_ip: String,
    pub public_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshJoinResponse {
    pub mesh_ip: String,
    pub subnet: String,
    pub gateway: String,
    pub peers: Vec<MeshPeer>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshHeartbeatPayload {
    pub node_id: String,
    pub public_ip: String,
    pub public_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshHeartbeatResponse {
    pub mesh_ip: String,
    pub peers: Vec<MeshPeer>,
}