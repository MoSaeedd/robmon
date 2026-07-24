use directories::ProjectDirs;
use log::{debug, info, warn};
use reqwest::Client;
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, io::{self, Write}, path::PathBuf, time::Duration};
use sysinfo::System;
use tokio::{process::Command, time};
use uuid::Uuid;

use robmon_agent::crypto::{self, WireGuardKeypair};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const AGENT_VERSION: &str = "0.1.0";

fn control_plane_url() -> String {
    let default_url = "http://127.0.0.1:8080".to_string();
    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == "--control-plane-url" {
            if let Some(url) = args.next() {
                return url;
            }
        } else if let Some(url) = arg.strip_prefix("--control-plane-url=") {
            return url.to_string();
        }
    }
    env::var("CONTROL_PLANE_URL").unwrap_or(default_url)
}

#[derive(Serialize, Deserialize, Debug)]
struct RobotMetadata {
    robot_id: String,
    hostname: String,
    os: String,
    arch: String,
    ros_version: String,
    agent_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoadAverage {
    one: f64,
    five: f64,
    fifteen: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SystemMetrics {
    cpu_usage: f32,
    cpu_cores: usize,
    memory_total_bytes: u64,
    memory_used_bytes: u64,
    memory_available_bytes: u64,
    load_average: LoadAverage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MeshPeer {
    node_id: String,
    hostname: String,
    mesh_ip: String,
    public_ip: String,
    public_port: u16,
    online: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct MeshState {
    mesh_ip: Option<String>,
    public_ip: Option<String>,
    public_port: Option<u16>,
    subnet: Option<String>,
    gateway: Option<String>,
    peers: Vec<MeshPeer>,
    #[serde(default)]
    public_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AgentState {
    metadata: RobotMetadata,
    metrics: SystemMetrics,
    last_seen: String,
    command_history: Vec<String>,
    #[serde(default)]
    mesh: MeshState,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandResponse {
    commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthState {
    token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceRegistration {
    #[serde(rename = "serviceName")]
    service_name: String,
    host: String,
    port: u16,
    protocol: String,
    meta: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshJoinPayload {
    node_id: String,
    hostname: String,
    public_ip: String,
    public_port: u16,
    #[serde(default)]
    public_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshJoinResponse {
    mesh_ip: String,
    subnet: String,
    gateway: String,
    peers: Vec<MeshPeer>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshHeartbeatPayload {
    node_id: String,
    public_ip: String,
    public_port: u16,
    #[serde(default)]
    public_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshHeartbeatResponse {
    mesh_ip: String,
    peers: Vec<MeshPeer>,
}

fn get_data_dir() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "robmon", "agent")
        .ok_or("Unable to derive XDG project directories")?;
    let data_dir = project_dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    Ok(data_dir.to_path_buf())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let data_dir = get_data_dir()?;
    let state_path = data_dir.join("agent_state.json");
    let mut agent_state = load_state(&state_path).unwrap_or_else(|err| {
        warn!("Unable to load saved state: {}. Creating new agent state.", err);
        create_initial_state()
    });

    // ── Mesh VPN: ensure a WireGuard keypair exists ─────────────────
    let mesh_keypair = ensure_mesh_keypair(&data_dir)?;
    println!(
        "🔑 Mesh VPN public key: {}",
        mesh_keypair.public_key_base64()
    );
    println!(
        "   Private key stored at: {:?}",
        data_dir.join("mesh_private_key.json")
    );

    let control_plane_url = control_plane_url();
    info!("RobMon agent starting: {}", agent_state.metadata.robot_id);
    info!("Using control plane: {}", control_plane_url);
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    if env::args().any(|arg| arg == "--logout") {
        return handle_logout(&client, &control_plane_url).await;
    }

    let token = ensure_login(&client, &control_plane_url).await?;
    if env::args().any(|arg| arg == "--login") {
        println!("Login successful. Token saved.");
        return Ok(());
    }

    // Store public key in mesh state for mesh join
    agent_state.mesh.public_key = Some(mesh_keypair.public_key_base64());

    maybe_register_service(&client, &control_plane_url, &token).await?;
    join_mesh(&client, &control_plane_url, &token, &mut agent_state).await?;

    let mut system = System::new_all();
    let mut loop_tick: u64 = 0;

    loop {
        let metrics = collect_system_metrics(&mut system);
        agent_state.metrics = metrics.clone();
        agent_state.last_seen = chrono::Utc::now().to_rfc3339();

        if loop_tick % 10 == 0 {
            if let Err(err) = refresh_mesh(&client, &control_plane_url, &token, &mut agent_state).await {
                warn!("Mesh sync failed: {}", err);
            }
        }
        loop_tick += 1;

        if let Err(err) = save_state(&state_path, &agent_state) {
            warn!("Failed to persist local state: {}", err);
        }

        if let Err(err) = publish_state(&client, &control_plane_url, &token, &agent_state).await {
            warn!("Control plane update failed: {}", err);
        }

        match fetch_commands(&client, &control_plane_url, &token, &agent_state.metadata.robot_id).await {
            Ok(commands) if !commands.is_empty() => {
                info!("Fetched {} command(s) from control plane", commands.len());
                for command in commands {
                    let result = execute_command(&command).await;
                    let history_entry = format!("{} => {}", command, result.trim());
                    debug!("Command result: {}", history_entry);
                    agent_state.command_history.push(history_entry);
                }
            }
            Ok(_) => debug!("No commands available from control plane.") ,
            Err(err) => warn!("Failed to fetch commands: {}", err),
        }

        time::sleep(Duration::from_secs(1)).await;
    }
}

/// Ensure a WireGuard keypair exists on disk, generating one if needed.
fn ensure_mesh_keypair(data_dir: &PathBuf) -> Result<WireGuardKeypair> {
    match crypto::load_keypair(data_dir)? {
        Some(kp) => {
            info!("Loaded existing mesh VPN keypair");
            println!("   ✅ Loaded existing mesh keypair");
            Ok(kp)
        }
        None => {
            info!("No mesh keypair found. Generating new Curve25519 keypair...");
            println!("   🔑 Generating new mesh VPN keypair...");
            let kp = WireGuardKeypair::generate();
            crypto::save_keypair(data_dir, &kp)?;
            println!("   ✅ New keypair saved to disk");
            Ok(kp)
        }
    }
}

fn create_initial_state() -> AgentState {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string());

    let metadata = RobotMetadata {
        robot_id: format!("{}-{}", hostname, Uuid::new_v4()),
        hostname,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        ros_version: "unknown".to_string(),
        agent_version: AGENT_VERSION.to_string(),
    };

    let mut system = System::new_all();
    AgentState {
        metadata,
        metrics: collect_system_metrics(&mut system),
        last_seen: chrono::Utc::now().to_rfc3339(),
        command_history: Vec::new(),
        mesh: MeshState::default(),
    }
}

fn get_state_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "robmon", "agent")
        .ok_or("Unable to derive XDG project directories")?;
    let data_dir = project_dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    Ok(data_dir.join("agent_state.json"))
}

fn get_token_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "robmon", "agent")
        .ok_or("Unable to derive XDG project directories")?;
    let data_dir = project_dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    Ok(data_dir.join("agent_token.json"))
}

fn load_auth_token(path: &PathBuf) -> Result<AuthState> {
    let contents = fs::read_to_string(path)?;
    let auth_state = serde_json::from_str(&contents)?;
    Ok(auth_state)
}

fn save_auth_token(path: &PathBuf, auth_state: &AuthState) -> Result<()> {
    let contents = serde_json::to_string_pretty(auth_state)?;
    fs::write(path, contents)?;
    Ok(())
}

fn prompt_login() -> Result<LoginPayload> {
    print!("Control plane username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Control plane password: ");
    io::stdout().flush()?;
    let password = read_password()?;
    Ok(LoginPayload { username, password })
}

async fn handle_logout(client: &Client, control_plane_url: &str) -> Result<()> {
    let token_path = get_token_path()?;
    match load_auth_token(&token_path) {
        Ok(state) => {
            let endpoint = format!("{}/api/logout", control_plane_url);
            let response = client.post(&endpoint).bearer_auth(state.token.clone()).send().await?;
            if response.status().is_success() {
                fs::remove_file(&token_path)?;
                println!("Logout successful.");
            } else {
                warn!("Logout request failed: {}", response.status());
            }
        }
        Err(err) => {
            warn!("No saved token to logout: {}", err);
        }
    }

    if token_path.exists() {
        fs::remove_file(&token_path)?;
    }

    Ok(())
}

async fn ensure_login(client: &Client, control_plane_url: &str) -> Result<String> {
    let token_path = get_token_path()?;
    if let Ok(state) = load_auth_token(&token_path) {
        return Ok(state.token);
    }

    let username = env::var("CONTROL_PLANE_USER").ok();
    let password = env::var("CONTROL_PLANE_PASSWORD").ok();
    let login = if let (Some(username), Some(password)) = (username, password) {
        LoginPayload { username, password }
    } else {
        prompt_login()?
    };

    let endpoint = format!("{}/api/login", control_plane_url);
    let response = client.post(&endpoint).json(&login).send().await?;
    if !response.status().is_success() {
        return Err(format!("Login failed: {}", response.status()).into());
    }

    let login_response: LoginResponse = response.json().await?;
    let auth_state = AuthState { token: login_response.access_token.clone() };
    save_auth_token(&token_path, &auth_state)?;
    println!("✅ Login successful. Token saved.");
    Ok(login_response.access_token)
}

async fn maybe_register_service(client: &Client, control_plane_url: &str, token: &str) -> Result<()> {
    let service_name = match env::var("SERVICE_NAME") {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let port = match env::var("SERVICE_PORT") {
        Ok(value) => value.parse::<u16>()?,
        Err(_) => return Ok(()),
    };
    let host = env::var("SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let protocol = env::var("SERVICE_PROTOCOL").unwrap_or_else(|_| "http".to_string());

    let registration = ServiceRegistration {
        service_name: service_name,
        host,
        port,
        protocol,
        meta: HashMap::new(),
    };

    let endpoint = format!("{}/api/services", control_plane_url);
    let response = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(&registration)
        .send()
        .await?;

    if response.status().is_success() {
        info!("Service registered successfully");
        Ok(())
    } else {
        warn!("Service registration failed: {}", response.status());
        Ok(())
    }
}

fn mesh_public_ip() -> Option<String> {
    env::var("MESH_PUBLIC_IP")
        .ok()
        .or_else(|| env::var("SERVICE_HOST").ok())
        .filter(|value| !value.is_empty())
}

fn mesh_public_port() -> u16 {
    env::var("MESH_PUBLIC_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(51820)
}

async fn join_mesh(
    client: &Client,
    control_plane_url: &str,
    token: &str,
    agent_state: &mut AgentState,
) -> Result<()> {
    let public_ip = match mesh_public_ip() {
        Some(ip) => ip,
        None => {
            warn!("Skipping mesh join: set MESH_PUBLIC_IP to advertise a reachable public address.");
            return Ok(());
        }
    };
    let public_port = mesh_public_port();

    let payload = MeshJoinPayload {
        node_id: agent_state.metadata.robot_id.clone(),
        hostname: agent_state.metadata.hostname.clone(),
        public_ip: public_ip.clone(),
        public_port,
        public_key: agent_state.mesh.public_key.clone(),
    };

    let endpoint = format!("{}/api/mesh/join", control_plane_url);
    let response = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("mesh join failed: {}", response.status()).into());
    }

    let mesh_response: MeshJoinResponse = response.json().await?;
    agent_state.mesh = MeshState {
        mesh_ip: Some(mesh_response.mesh_ip.clone()),
        public_ip: Some(public_ip),
        public_port: Some(public_port),
        subnet: Some(mesh_response.subnet),
        gateway: Some(mesh_response.gateway),
        peers: mesh_response.peers,
        public_key: agent_state.mesh.public_key.clone(),
    };

    info!(
        "Joined mesh at {} with {} peer(s)",
        mesh_response.mesh_ip,
        agent_state.mesh.peers.len()
    );
    Ok(())
}

async fn refresh_mesh(
    client: &Client,
    control_plane_url: &str,
    token: &str,
    agent_state: &mut AgentState,
) -> Result<()> {
    if agent_state.mesh.mesh_ip.is_none() {
        return join_mesh(client, control_plane_url, token, agent_state).await;
    }

    let public_ip = mesh_public_ip().unwrap_or_default();
    let public_port = mesh_public_port();
    let payload = MeshHeartbeatPayload {
        node_id: agent_state.metadata.robot_id.clone(),
        public_ip,
        public_port,
        public_key: agent_state.mesh.public_key.clone(),
    };

    let endpoint = format!("{}/api/mesh/heartbeat", control_plane_url);
    let response = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?;

    if response.status().as_u16() == 404 {
        return join_mesh(client, control_plane_url, token, agent_state).await;
    }

    if !response.status().is_success() {
        return Err(format!("mesh heartbeat failed: {}", response.status()).into());
    }

    let heartbeat: MeshHeartbeatResponse = response.json().await?;
    let previous_peer_count = agent_state.mesh.peers.len();
    agent_state.mesh.mesh_ip = Some(heartbeat.mesh_ip);
    agent_state.mesh.public_ip = mesh_public_ip();
    agent_state.mesh.public_port = Some(public_port);
    agent_state.mesh.peers = heartbeat.peers;

    if agent_state.mesh.peers.len() != previous_peer_count {
        info!(
            "Mesh peer list updated: {} online peer(s)",
            agent_state.mesh.peers.len()
        );
        for peer in &agent_state.mesh.peers {
            info!(
                "  peer {} ({}) -> mesh {} via {}:{}",
                peer.hostname, peer.node_id, peer.mesh_ip, peer.public_ip, peer.public_port
            );
        }
    }

    Ok(())
}

fn load_state(path: &PathBuf) -> Result<AgentState> {
    let contents = fs::read_to_string(path)?;
    let state = serde_json::from_str(&contents)?;
    Ok(state)
}

fn save_state(path: &PathBuf, state: &AgentState) -> Result<()> {
    let contents = serde_json::to_string_pretty(state)?;
    fs::write(path, contents)?;
    Ok(())
}

fn collect_system_metrics(system: &mut System) -> SystemMetrics {
    system.refresh_cpu_all();
    system.refresh_memory();

    let cpu_usage = system
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage())
        .sum::<f32>()
        / system.cpus().len().max(1) as f32;

    SystemMetrics {
        cpu_usage,
        cpu_cores: system.cpus().len(),
        memory_total_bytes: system.total_memory(),
        memory_used_bytes: system.used_memory(),
        memory_available_bytes: system.available_memory(),
        load_average: LoadAverage {
            one: System::load_average().one,
            five: System::load_average().five,
            fifteen: System::load_average().fifteen,
        },
    }
}

async fn publish_state(client: &Client, control_plane_url: &str, token: &str, state: &AgentState) -> Result<()> {
    let endpoint = format!("{}/api/robots", control_plane_url);
    let response = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(state)
        .send()
        .await?;
    if response.status().is_success() {
        info!("Robot state published successfully to control plane.");
        Ok(())
    } else {
        Err(format!("publish failed: {}", response.status()).into())
    }
}

async fn fetch_commands(client: &Client, control_plane_url: &str, token: &str, robot_id: &str) -> Result<Vec<String>> {
    let endpoint = format!("{}/api/robots/{}/commands", control_plane_url, robot_id);
    let response = client.get(&endpoint).bearer_auth(token).send().await?;
    if response.status().is_success() {
        let command_response: CommandResponse = response.json().await?;
        Ok(command_response.commands)
    } else if response.status().as_u16() == 404 {
        Ok(Vec::new())
    } else {
        Err(format!("command fetch failed: {}", response.status()).into())
    }
}

async fn execute_command(command: &str) -> String {
    info!("Executing command: {}", command);
    let output = Command::new("sh").arg("-c").arg(command).output().await;
    match output {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                format!("error: {}", String::from_utf8_lossy(&output.stderr))
            }
        }
        Err(err) => format!("failed to spawn command: {}", err),
    }
}