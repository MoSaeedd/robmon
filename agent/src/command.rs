use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

pub async fn execute_command(command: &str) -> String {
    info!("Executing command: {}", command);
    
    let output = TokioCommand::new("sh").arg("-c").arg(command).output().await;
    
    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                info!("Command succeeded: {}", stdout.trim());
                stdout
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                warn!("Command failed: {}", stderr.trim());
                format!("error: {}", stderr)
            }
        }
        Err(err) => {
            warn!("Failed to spawn command: {}", err);
            format!("failed to spawn command: {}", err)
        }
    }
}