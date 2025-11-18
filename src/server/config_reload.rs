use crate::config::Config;
use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

/// Configuration reload manager
pub struct ConfigReloadManager {
    config_path: PathBuf,
    current_config: Arc<RwLock<Config>>,
}

impl ConfigReloadManager {
    pub fn new(config_path: PathBuf, initial_config: Config) -> Self {
        Self {
            config_path,
            current_config: Arc::new(RwLock::new(initial_config)),
        }
    }

    /// Get current configuration (read-only access)
    pub fn config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.current_config)
    }

    /// Reload configuration from file
    pub fn reload(&self) -> Result<()> {
        info!("Reloading configuration from {:?}", self.config_path);

        // Load new configuration
        let new_config = Config::from_file(&self.config_path)
            .with_context(|| format!("Failed to load config from {:?}", self.config_path))?;

        // Validate using built-in validation
        match new_config.validate() {
            Ok(warnings) => {
                if !warnings.is_empty() {
                    info!("Configuration warnings: {:?}", warnings);
                }
            }
            Err(e) => {
                anyhow::bail!("Configuration validation failed: {}", e);
            }
        }

        // Update current configuration
        {
            let mut config = self.current_config.write();
            *config = new_config;
        }

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Start listening for SIGUSR1 signal
    pub async fn start_signal_handler(self: Arc<Self>) -> Result<()> {
        let mut usr1 = signal(SignalKind::user_defined1())
            .context("Failed to register SIGUSR1 handler")?;

        info!("Configuration reload signal handler started (send SIGUSR1 to reload)");

        loop {
            usr1.recv().await;
            info!("Received SIGUSR1 signal, triggering config reload");

            if let Err(e) = self.reload() {
                error!("Failed to reload configuration: {}", e);
                // Don't crash, just log the error and continue
            }
        }
    }
}

/// Hot-reloadable configuration wrapper
///
/// This wrapper provides thread-safe access to configuration that can be
/// reloaded at runtime without restarting the server.
#[derive(Clone)]
pub struct HotReloadConfig {
    inner: Arc<RwLock<Config>>,
}

impl HotReloadConfig {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }

    pub fn from_manager(manager: &ConfigReloadManager) -> Self {
        Self {
            inner: Arc::clone(&manager.current_config),
        }
    }

    /// Read configuration (returns a snapshot)
    pub fn read(&self) -> Config {
        self.inner.read().clone()
    }

    /// Execute a function with read-only access to config
    pub fn with_config<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Config) -> R,
    {
        let config = self.inner.read();
        f(&config)
    }

    /// Get specific configuration value
    pub fn get_server_port(&self) -> u16 {
        self.inner.read().server.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_hot_reload_config() {
        // Load initial valid config from example
        let config_path = PathBuf::from("config/fe-php.example.toml");
        let config = Config::from_file(&config_path).expect("Failed to load example config");
        let hot_config = HotReloadConfig::new(config);

        // Read config
        let port = hot_config.get_server_port();
        assert_eq!(port, 8080); // Default port

        // Use with_config
        hot_config.with_config(|cfg| {
            assert_eq!(cfg.server.port, 8080);
        });
    }

    #[test]
    fn test_config_reload_file_not_found() {
        // Load initial valid config from example
        let config_path = PathBuf::from("config/fe-php.example.toml");
        let config = Config::from_file(&config_path).expect("Failed to load example config");

        let manager = ConfigReloadManager::new(
            PathBuf::from("/nonexistent/config.toml"),
            config,
        );

        // Should fail to reload
        assert!(manager.reload().is_err());
    }
}
