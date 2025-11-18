use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info};

use crate::admin::api::AdminApi;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Status,
    Health,
    Metrics,
    Analysis,  // ログ解析結果を取得
    BlockedIps,  // ブロックされているIPリスト取得
    ReloadConfig { config_path: Option<String> },
    RestartWorkers,
    BlockIp { ip: String },
    UnblockIp { ip: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            status: "ok".to_string(),
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            status: "error".to_string(),
            data: None,
            error: Some(message),
        }
    }
}

pub struct UnixSocketServer {
    socket_path: PathBuf,
    admin_api: Arc<AdminApi>,
}

impl UnixSocketServer {
    pub fn new(socket_path: PathBuf, admin_api: Arc<AdminApi>) -> Self {
        Self {
            socket_path,
            admin_api,
        }
    }

    pub async fn serve(&self) -> Result<()> {
        // Remove old socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .context("Failed to remove old socket file")?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .context("Failed to bind Unix socket")?;

        // Set socket permissions (0600 - owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.socket_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.socket_path, perms)?;
        }

        info!("Unix socket server listening on {:?}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let admin_api = self.admin_api.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, admin_api).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(stream: UnixStream, admin_api: Arc<AdminApi>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;

        if n == 0 {
            // Connection closed
            debug!("Client disconnected");
            break;
        }

        let response = match process_command(&line.trim(), &admin_api).await {
            Ok(resp) => resp,
            Err(e) => Response::error(format!("{}", e)),
        };

        let json = serde_json::to_string(&response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn process_command(line: &str, admin_api: &AdminApi) -> Result<Response> {
    // Try to parse as JSON command first
    let command: Command = if line.starts_with('{') {
        serde_json::from_str(line)?
    } else {
        // Simple text protocol fallback
        match line.to_lowercase().as_str() {
            "status" => Command::Status,
            "health" => Command::Health,
            "metrics" => Command::Metrics,
            "analysis" => Command::Analysis,
            "blocked_ips" | "blocked" => Command::BlockedIps,
            cmd if cmd.starts_with("reload") => Command::ReloadConfig {
                config_path: None,
            },
            cmd if cmd.starts_with("restart") => Command::RestartWorkers,
            cmd if cmd.starts_with("block ") => {
                let ip = cmd.strip_prefix("block ").unwrap_or("").trim().to_string();
                Command::BlockIp { ip }
            }
            cmd if cmd.starts_with("unblock ") => {
                let ip = cmd.strip_prefix("unblock ").unwrap_or("").trim().to_string();
                Command::UnblockIp { ip }
            }
            _ => {
                return Ok(Response::error(format!("Unknown command: {}", line)));
            }
        }
    };

    execute_command(command, admin_api).await
}

async fn execute_command(command: Command, admin_api: &AdminApi) -> Result<Response> {
    match command {
        Command::Status => {
            let status = admin_api.get_status();
            Ok(Response::success(serde_json::to_value(status)?))
        }
        Command::Health => {
            let health = admin_api.health_check();
            Ok(Response::success(serde_json::to_value(health)?))
        }
        Command::Metrics => {
            let metrics = admin_api.get_metrics_text();
            Ok(Response::success(serde_json::json!({
                "prometheus": metrics
            })))
        }
        Command::Analysis => {
            let analysis = admin_api.get_log_analysis();
            Ok(Response::success(serde_json::to_value(analysis)?))
        }
        Command::BlockedIps => {
            let blocked_ips = admin_api.get_blocked_ips();
            Ok(Response::success(serde_json::json!({
                "blocked_ips": blocked_ips,
                "count": blocked_ips.len()
            })))
        }
        Command::ReloadConfig { config_path } => {
            match admin_api.reload_config() {
                Ok(()) => Ok(Response::success(serde_json::json!({
                    "message": "Configuration reload request sent",
                    "config_path": config_path,
                }))),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Command::RestartWorkers => {
            match admin_api.restart_workers() {
                Ok(()) => Ok(Response::success(serde_json::json!({
                    "message": "Worker restart request sent"
                }))),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Command::BlockIp { ip } => {
            match admin_api.block_ip(ip.clone()) {
                Ok(()) => Ok(Response::success(serde_json::json!({
                    "message": format!("IP {} block request sent", ip)
                }))),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Command::UnblockIp { ip } => {
            match admin_api.unblock_ip(ip.clone()) {
                Ok(()) => Ok(Response::success(serde_json::json!({
                    "message": format!("IP {} unblock request sent", ip)
                }))),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        // Cleanup socket file on drop
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}
