use clap::Parser;
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Control plane URL to connect to
    #[arg(long, env = "CONTROL_PLANE_URL", default_value = "http://127.0.0.1:8080")]
    pub control_plane_url: String,

    /// Public IP for mesh networking
    #[arg(long, env = "MESH_PUBLIC_IP")]
    pub mesh_public_ip: Option<String>,

    /// Public port for mesh networking
    #[arg(long, env = "MESH_PUBLIC_PORT", default_value_t = 51820)]
    pub mesh_public_port: u16,

    /// Control plane username for authentication
    #[arg(long, env = "CONTROL_PLANE_USER")]
    pub control_plane_user: Option<String>,

    /// Control plane password for authentication
    #[arg(long, env = "CONTROL_PLANE_PASSWORD")]
    pub control_plane_password: Option<String>,

    /// Service name for registration
    #[arg(long, env = "SERVICE_NAME")]
    pub service_name: Option<String>,

    /// Service port for registration
    #[arg(long, env = "SERVICE_PORT")]
    pub service_port: Option<u16>,

    /// Service host for registration
    #[arg(long, env = "SERVICE_HOST")]
    pub service_host: Option<String>,

    /// Service protocol for registration
    #[arg(long, env = "SERVICE_PROTOCOL", default_value = "http")]
    pub service_protocol: String,

    /// Heartbeat interval in seconds
    #[arg(long, env = "HEARTBEAT_INTERVAL", default_value_t = 10)]
    pub heartbeat_interval: u64,

    /// Metrics collection interval in seconds
    #[arg(long, env = "METRICS_INTERVAL", default_value_t = 1)]
    pub metrics_interval: u64,

    /// Logout and exit
    #[arg(long)]
    pub logout: bool,

    /// Login and exit
    #[arg(long)]
    pub login: bool,
}

impl Config {
    pub fn heartbeat_duration(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval)
    }

    pub fn metrics_duration(&self) -> Duration {
        Duration::from_secs(self.metrics_interval)
    }
}