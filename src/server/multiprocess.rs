use anyhow::{Context, Result};
use std::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Multi-process manager for running fe-php in multiple processes
pub struct MultiProcessManager {
    process_count: usize,
    workers: Arc<RwLock<Vec<WorkerProcess>>>,
}

impl MultiProcessManager {
    /// Create a new multi-process manager
    pub fn new(process_count: usize) -> Self {
        Self {
            process_count,
            workers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start all worker processes
    pub async fn start(&self, config_path: &str) -> Result<()> {
        info!("Starting {} worker processes", self.process_count);

        let mut workers = self.workers.write().await;

        for i in 0..self.process_count {
            let worker = WorkerProcess::spawn(i, config_path)?;
            workers.push(worker);
        }

        debug!("All worker processes started");
        Ok(())
    }

    /// Stop all worker processes
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping all worker processes");

        let mut workers = self.workers.write().await;

        for worker in workers.iter_mut() {
            worker.stop()?;
        }

        workers.clear();
        debug!("All worker processes stopped");
        Ok(())
    }

    /// Restart a specific worker process
    pub async fn restart_worker(&self, worker_id: usize, config_path: &str) -> Result<()> {
        info!("Restarting worker process {}", worker_id);

        let mut workers = self.workers.write().await;

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.stop()?;
            *worker = WorkerProcess::spawn(worker_id, config_path)?;
            debug!("Worker process {} restarted", worker_id);
            Ok(())
        } else {
            anyhow::bail!("Worker {} not found", worker_id)
        }
    }

    /// Get status of all worker processes
    pub async fn get_status(&self) -> Vec<WorkerStatus> {
        let workers = self.workers.read().await;
        workers.iter().map(|w| w.get_status()).collect()
    }

    /// Monitor worker processes and restart if they crash
    pub async fn monitor(self: Arc<Self>, config_path: String) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let workers = self.workers.read().await;
                for (i, worker) in workers.iter().enumerate() {
                    if !worker.is_alive() {
                        warn!("Worker process {} is dead, will restart", i);
                        drop(workers); // Release read lock

                        if let Err(e) = self.restart_worker(i, &config_path).await {
                            error!("Failed to restart worker {}: {}", i, e);
                        }

                        break; // Re-acquire lock in next iteration
                    }
                }
            }
        });
    }
}

/// Represents a worker process
struct WorkerProcess {
    id: usize,
    child: Option<Child>,
}

impl WorkerProcess {
    /// Spawn a new worker process
    fn spawn(id: usize, config_path: &str) -> Result<Self> {
        // Get the current executable path
        let exe_path = std::env::current_exe()
            .context("Failed to get current executable path")?;

        // Spawn a child process with the same configuration
        let child = Command::new(exe_path)
            .arg("serve")
            .arg("--config")
            .arg(config_path)
            .env("FE_PHP_WORKER_ID", id.to_string())
            .spawn()
            .context("Failed to spawn worker process")?;

        debug!("Spawned worker process {} with PID {}", id, child.id());

        Ok(Self {
            id,
            child: Some(child),
        })
    }

    /// Stop the worker process
    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            debug!("Stopping worker process {}", self.id);

            // Send SIGTERM on Unix, kill on Windows
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(child.id() as i32);
                if let Err(e) = kill(pid, Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to worker {}: {}", self.id, e);
                    child.kill().context("Failed to kill worker process")?;
                }
            }

            #[cfg(not(unix))]
            {
                child.kill().context("Failed to kill worker process")?;
            }

            child.wait().context("Failed to wait for worker process")?;
        }

        Ok(())
    }

    /// Check if the worker process is alive
    fn is_alive(&self) -> bool {
        if let Some(child) = &self.child {
            // On Unix, we can check if the process exists without blocking
            #[cfg(unix)]
            {
                use nix::sys::signal::kill;
                use nix::unistd::Pid;

                let pid = Pid::from_raw(child.id() as i32);
                // Send signal 0 (None) to check if process exists
                kill(pid, None).is_ok()
            }

            #[cfg(not(unix))]
            {
                // On Windows, assume alive if we have a handle
                true
            }
        } else {
            false
        }
    }

    /// Get the status of the worker process
    fn get_status(&self) -> WorkerStatus {
        WorkerStatus {
            id: self.id,
            pid: self.child.as_ref().map(|c| c.id()),
            alive: self.is_alive(),
        }
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub id: usize,
    pub pid: Option<u32>,
    pub alive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multiprocess_manager_creation() {
        let manager = MultiProcessManager::new(4);
        assert_eq!(manager.process_count, 4);
    }
}
