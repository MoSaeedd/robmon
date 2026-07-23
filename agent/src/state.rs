use crate::error::{AgentError, Result};
use crate::models::{AgentState, AuthState};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug)]
pub struct StateManager {
    state_path: PathBuf,
    token_path: PathBuf,
    agent_state: RwLock<AgentState>,
}

impl StateManager {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "robmon", "agent")
            .ok_or_else(|| AgentError::StateError("Failed to get project directories".to_string()))?;

        let data_dir = project_dirs.data_dir();
        fs::create_dir_all(data_dir)?;

        let state_path = data_dir.join("agent_state.json");
        let token_path = data_dir.join("agent_token.json");

        let agent_state = fs::read_to_string(&state_path)
            .and_then(|contents| {
                serde_json::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })
            .unwrap_or_else(|err| {
                warn!("Failed to load existing state, creating new: {}", err);
                AgentState::default()
            });

        info!("State manager initialized. Robot ID: {}", agent_state.metadata.robot_id);

        Ok(Self {
            state_path,
            token_path,
            agent_state: RwLock::new(agent_state),
        })
    }

    pub async fn get_state(&self) -> tokio::sync::RwLockReadGuard<'_, AgentState> {
        self.agent_state.read().await
    }

    pub async fn update_state<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut AgentState),
    {
        let mut state = self.agent_state.write().await;
        update_fn(&mut state);
        state.last_seen = chrono::Utc::now();
        self.save_state(&state).await?;
        Ok(())
    }

    async fn save_state(&self, state: &AgentState) -> Result<()> {
        let contents = serde_json::to_string_pretty(state)?;
        fs::write(&self.state_path, contents)?;
        Ok(())
    }

    pub fn load_auth_token(&self) -> Result<AuthState> {
        let contents = fs::read_to_string(&self.token_path)?;
        let auth_state = serde_json::from_str(&contents)?;
        Ok(auth_state)
    }

    pub fn save_auth_token(&self, auth_state: &AuthState) -> Result<()> {
        let contents = serde_json::to_string_pretty(auth_state)?;
        fs::write(&self.token_path, contents)?;
        Ok(())
    }

    pub fn clear_auth_token(&self) -> Result<()> {
        if self.token_path.exists() {
            fs::remove_file(&self.token_path)?;
        }
        Ok(())
    }

    pub fn token_path_exists(&self) -> bool {
        self.token_path.exists()
    }

    pub fn robot_id(&self) -> String {
        if let Ok(state) = self.agent_state.try_read() {
            state.metadata.robot_id.clone()
        } else {
            "unknown".to_string()
        }
    }
}