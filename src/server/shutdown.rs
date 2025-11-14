use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Shutdown coordinator for graceful server shutdown
pub struct ShutdownCoordinator {
    /// Broadcast channel to notify all tasks of shutdown
    shutdown_tx: broadcast::Sender<()>,
    /// Flag indicating if shutdown has been initiated
    is_shutting_down: Arc<AtomicBool>,
    /// Number of active connections
    active_connections: Arc<AtomicUsize>,
    /// Graceful shutdown timeout
    timeout: Duration,
}

impl ShutdownCoordinator {
    pub fn new(timeout_secs: u64) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);

        Self {
            shutdown_tx,
            is_shutting_down: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Get a shutdown receiver
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::SeqCst)
    }

    /// Increment active connection counter
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active connection counter
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get current active connection count
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Initiate graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown...");

        // Set shutdown flag
        self.is_shutting_down.store(true, Ordering::SeqCst);

        // Broadcast shutdown signal to all tasks
        let _ = self.shutdown_tx.send(());

        // Wait for active connections to complete
        self.wait_for_connections().await
    }

    async fn wait_for_connections(&self) -> Result<()> {
        let start = Instant::now();

        loop {
            let active = self.active_connections.load(Ordering::SeqCst);

            if active == 0 {
                info!("All connections closed gracefully");
                return Ok(());
            }

            if start.elapsed() > self.timeout {
                warn!(
                    "Graceful shutdown timeout ({} seconds) reached with {} active connections, forcing shutdown",
                    self.timeout.as_secs(),
                    active
                );
                return Ok(());
            }

            info!(
                "Waiting for {} active connection(s) to complete... ({:.1}s remaining)",
                active,
                (self.timeout - start.elapsed()).as_secs_f64()
            );

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Setup signal handlers for graceful shutdown
pub async fn setup_signal_handler(coordinator: Arc<ShutdownCoordinator>) {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate())
        .expect("Failed to setup SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt())
        .expect("Failed to setup SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM signal");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT signal (Ctrl+C)");
        }
    }

    // Initiate graceful shutdown
    if let Err(e) = coordinator.shutdown().await {
        warn!("Error during graceful shutdown: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let coordinator = ShutdownCoordinator::new(5);

        assert!(!coordinator.is_shutting_down());
        assert_eq!(coordinator.active_connections(), 0);

        coordinator.inc_connections();
        assert_eq!(coordinator.active_connections(), 1);

        coordinator.dec_connections();
        assert_eq!(coordinator.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_shutdown_with_no_connections() {
        let coordinator = ShutdownCoordinator::new(5);

        let result = coordinator.shutdown().await;
        assert!(result.is_ok());
        assert!(coordinator.is_shutting_down());
    }
}
