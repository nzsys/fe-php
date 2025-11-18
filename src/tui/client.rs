use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Status,
    Health,
    Metrics,
    Analysis,
    BlockedIps,
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

/// Unix Socket client for TUI to communicate with running server
pub struct TuiClient {
    socket_path: PathBuf,
}

impl TuiClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Connect to the Unix socket with timeout
    async fn connect(&self) -> Result<UnixStream> {
        use tokio::time::{timeout, Duration};

        // Try to connect with a 5-second timeout
        let connect_future = UnixStream::connect(&self.socket_path);

        match timeout(Duration::from_secs(5), connect_future).await {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(anyhow::anyhow!(
                "Failed to connect to Unix socket {:?}: {}. Is the server running?",
                self.socket_path,
                e
            )),
            Err(_) => Err(anyhow::anyhow!(
                "Connection timeout while connecting to {:?}. Is the server running?",
                self.socket_path
            )),
        }
    }

    /// Check if server is reachable
    pub async fn is_reachable(&self) -> bool {
        self.connect().await.is_ok()
    }

    /// Send a command and receive response with retry logic
    async fn send_command(&self, command: Command) -> Result<Response> {
        use tokio::time::{timeout, Duration};

        let stream = self.connect().await?;
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send command as JSON with timeout
        let command_json = serde_json::to_string(&command)?;

        match timeout(Duration::from_secs(3), async {
            writer.write_all(command_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to send command: {}", e)),
            Err(_) => return Err(anyhow::anyhow!("Timeout while sending command")),
        }

        // Read response with timeout
        let mut line = String::new();

        match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err(anyhow::anyhow!("Server closed connection unexpectedly"));
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Failed to read response: {}", e));
            }
            Err(_) => {
                return Err(anyhow::anyhow!("Timeout while reading response"));
            }
        }

        let response: Response = serde_json::from_str(&line.trim()).with_context(|| {
            format!(
                "Failed to parse response from server. Received: {}",
                line.trim()
            )
        })?;

        Ok(response)
    }

    /// Get server status
    pub async fn get_status(&self) -> Result<crate::admin::api::ServerStatus> {
        let response = self.send_command(Command::Status).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let status = serde_json::from_value(response.data.unwrap_or_default())
            .context("Failed to parse server status")?;

        Ok(status)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let response = self.send_command(Command::Health).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        Ok(response.data.unwrap_or_default())
    }

    /// Get metrics in Prometheus format
    pub async fn get_metrics(&self) -> Result<String> {
        let response = self.send_command(Command::Metrics).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let metrics = response
            .data
            .and_then(|v| v.get("prometheus").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or_default();

        Ok(metrics)
    }

    /// Get log analysis
    pub async fn get_analysis(&self) -> Result<crate::monitor::analyzer::LogAnalysisResult> {
        let response = self.send_command(Command::Analysis).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let analysis = serde_json::from_value(response.data.unwrap_or_default())
            .context("Failed to parse log analysis result")?;

        Ok(analysis)
    }

    /// Reload configuration
    pub async fn reload_config(&self, config_path: Option<String>) -> Result<String> {
        let response = self.send_command(Command::ReloadConfig { config_path }).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let message = response
            .data
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or_else(|| "Configuration reloaded".to_string());

        Ok(message)
    }

    /// Restart workers
    pub async fn restart_workers(&self) -> Result<String> {
        let response = self.send_command(Command::RestartWorkers).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let message = response
            .data
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or_else(|| "Workers restarted".to_string());

        Ok(message)
    }

    /// Block IP address
    pub async fn block_ip(&self, ip: String) -> Result<String> {
        let response = self.send_command(Command::BlockIp { ip: ip.clone() }).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let message = response
            .data
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or_else(|| format!("IP {} blocked", ip));

        Ok(message)
    }

    /// Unblock IP address
    pub async fn unblock_ip(&self, ip: String) -> Result<String> {
        let response = self.send_command(Command::UnblockIp { ip: ip.clone() }).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let message = response
            .data
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or_else(|| format!("IP {} unblocked", ip));

        Ok(message)
    }

    /// Get list of blocked IPs
    pub async fn get_blocked_ips(&self) -> Result<Vec<String>> {
        let response = self.send_command(Command::BlockedIps).await?;

        if response.status != "ok" {
            anyhow::bail!("Server returned error: {:?}", response.error);
        }

        let blocked_ips = response
            .data
            .and_then(|v| {
                v.get("blocked_ips").and_then(|ips| {
                    ips.as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|ip| ip.as_str().map(String::from))
                                .collect()
                        })
                })
            })
            .unwrap_or_default();

        Ok(blocked_ips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_connection() {
        // This is a placeholder test
        // In real scenarios, you would set up a test Unix socket server
        let client = TuiClient::new(PathBuf::from("/tmp/test.sock"));
        assert_eq!(client.socket_path, PathBuf::from("/tmp/test.sock"));
    }
}
