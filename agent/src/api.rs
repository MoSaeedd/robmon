use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::models::{
    AgentState, AuthState, CommandResponse, LoginPayload, LoginResponse, MeshHeartbeatPayload,
    MeshHeartbeatResponse, MeshJoinPayload, MeshJoinResponse, ServiceRegistration,
};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct ApiClient {
    client: Client,
    config: Config,
}

impl ApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        Ok(Self { client, config })
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        let endpoint = format!("{}/api/login", self.config.control_plane_url);
        let payload = LoginPayload {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        if !response.status().is_success() {
            error!("Login failed with status: {}", response.status());
            return Err(crate::error::AgentError::AuthenticationFailed(format!(
                "Login failed: {}",
                response.status()
            )));
        }

        let login_response: LoginResponse = response
            .json()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        info!("Successfully authenticated with control plane");
        Ok(login_response.access_token)
    }

    pub async fn logout(&self, auth_state: &AuthState) -> Result<()> {
        let endpoint = format!("{}/api/logout", self.config.control_plane_url);
        let _ = self
            .client
            .post(&endpoint)
            .bearer_auth(&auth_state.token)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        Ok(())
    }

    pub async fn register_service(&self, token: &str, service_name: &str, port: u16) -> Result<()> {
        let host = self
            .config
            .service_host
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let registration = ServiceRegistration {
            service_name: service_name.to_string(),
            host,
            port,
            protocol: self.config.service_protocol.clone(),
            meta: Default::default(),
        };

        let endpoint = format!("{}/api/services", self.config.control_plane_url);
        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(token)
            .json(&registration)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        if response.status().is_success() {
            info!("Service registered successfully");
        } else {
            warn!("Service registration failed: {}", response.status());
        }

        Ok(())
    }

    pub async fn join_mesh(
        &self,
        token: &str,
        node_id: &str,
        hostname: &str,
        public_ip: &str,
        public_port: u16,
    ) -> Result<MeshJoinResponse> {
        let payload = MeshJoinPayload {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            public_ip: public_ip.to_string(),
            public_port,
        };

        let endpoint = format!("{}/api/mesh/join", self.config.control_plane_url);
        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::error::AgentError::MeshError(format!(
                "Mesh join failed: {}",
                response.status()
            )));
        }

        let mesh_response: MeshJoinResponse = response
            .json()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        info!(
            "Joined mesh successfully. Assigned IP: {}, found {} peers",
            mesh_response.mesh_ip,
            mesh_response.peers.len()
        );

        Ok(mesh_response)
    }

    pub async fn send_mesh_heartbeat(
        &self,
        token: &str,
        node_id: &str,
        public_ip: &str,
        public_port: u16,
    ) -> Result<MeshHeartbeatResponse> {
        let payload = MeshHeartbeatPayload {
            node_id: node_id.to_string(),
            public_ip: public_ip.to_string(),
            public_port,
        };

        let endpoint = format!("{}/api/mesh/heartbeat", self.config.control_plane_url);
        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::error::AgentError::MeshError(format!(
                "Mesh heartbeat failed: {}",
                response.status()
            )));
        }

        let heartbeat: MeshHeartbeatResponse = response
            .json()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        debug!("Mesh heartbeat successful. {} peers online", heartbeat.peers.len());

        Ok(heartbeat)
    }

    pub async fn publish_state(&self, token: &str, state: &AgentState) -> Result<()> {
        let endpoint = format!("{}/api/robots", self.config.control_plane_url);
        let _ = self
            .client
            .post(&endpoint)
            .bearer_auth(token)
            .json(state)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        Ok(())
    }

    pub async fn fetch_commands(&self, token: &str, robot_id: &str) -> Result<Vec<String>> {
        let endpoint = format!("{}/api/robots/{}/commands", self.config.control_plane_url, robot_id);
        let response = self
            .client
            .get(&endpoint)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;

        if response.status().as_u16() == 404 {
            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            return Err(crate::error::AgentError::NetworkError(format!(
                "Failed to fetch commands: {}",
                response.status()
            ).into()));
        }

        let command_response: CommandResponse = response
            .json()
            .await
            .map_err(|err| AgentError::NetworkError(err.to_string()))?;
        Ok(command_response.commands)
    }
}