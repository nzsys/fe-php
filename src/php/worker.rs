use super::executor::{PhpExecutor, PhpRequest, PhpResponse};
use super::PhpConfig;
use anyhow::Result;
use async_channel::{Sender, Receiver, bounded};
use tokio::task;
use tracing::{info, warn, error};

pub struct WorkerPoolConfig {
    pub pool_size: usize,
    pub max_requests: usize,
}

pub struct WorkerPool {
    request_tx: Sender<(PhpRequest, Sender<Result<PhpResponse>>)>,
    config: WorkerPoolConfig,
    _php_module: Option<PhpExecutor>,  // Keep PHP module initialized for process lifetime
}

impl WorkerPool {
    pub fn new(php_config: PhpConfig, config: WorkerPoolConfig) -> Result<Self> {
        let (request_tx, request_rx) = bounded(config.pool_size * 2);

        // Initialize PHP module ONCE globally (not in worker threads)
        // This prevents "zend_mm_heap corrupted" error when multiple workers
        // try to call php_module_startup() simultaneously
        let php_module = if !php_config.use_fpm {
            Some(PhpExecutor::new(php_config.clone())?)  // Calls module_startup() once
        } else {
            None  // PHP-FPM mode doesn't need global initialization
        };

        // Spawn worker threads
        for worker_id in 0..config.pool_size {
            let request_rx = request_rx.clone();
            let php_config = php_config.clone();
            let max_requests = config.max_requests;

            task::spawn_blocking(move || {
                Self::worker_thread(worker_id, request_rx, php_config, max_requests);
            });
        }

        info!("Started PHP worker pool with {} workers", config.pool_size);

        Ok(Self {
            request_tx,
            config,
            _php_module: php_module,  // Kept alive for process lifetime
        })
    }

    fn worker_thread(
        worker_id: usize,
        request_rx: Receiver<(PhpRequest, Sender<Result<PhpResponse>>)>,
        php_config: PhpConfig,
        max_requests: usize,
    ) {
        info!("Worker {} started", worker_id);

        // Initialize PHP executor for this worker
        // Use new_worker() to skip module_startup (already called globally)
        let executor = match PhpExecutor::new_worker(php_config) {
            Ok(exec) => exec,
            Err(e) => {
                error!("Worker {} failed to initialize PHP: {}", worker_id, e);
                return;
            }
        };

        let mut requests_handled = 0;

        // Process requests until max_requests reached or channel closed
        while let Ok((request, response_tx)) = request_rx.recv_blocking() {
            let result = executor.execute(request);

            if let Err(e) = response_tx.send_blocking(result) {
                warn!("Worker {} failed to send response: {}", worker_id, e);
            }

            requests_handled += 1;

            // Restart worker after max_requests (prevent memory leaks)
            if max_requests > 0 && requests_handled >= max_requests {
                info!(
                    "Worker {} reached max requests ({}), restarting",
                    worker_id, max_requests
                );
                break;
            }
        }

        info!("Worker {} shutting down after {} requests", worker_id, requests_handled);
    }

    pub async fn execute(&self, request: PhpRequest) -> Result<PhpResponse> {
        let (response_tx, response_rx) = bounded(1);

        self.request_tx
            .send((request, response_tx))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request to worker pool: {}", e))?;

        response_rx
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response from worker: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[ignore] // Requires libphp.so
    fn test_worker_pool_creation() {
        let php_config = PhpConfig {
            libphp_path: PathBuf::from("/usr/local/lib/libphp.so"),
            document_root: PathBuf::from("/var/www/html"),
            worker_pool_size: 2,
            worker_max_requests: 1000,
        };

        let pool_config = WorkerPoolConfig {
            pool_size: 2,
            max_requests: 1000,
        };

        let result = WorkerPool::new(php_config, pool_config);
        assert!(result.is_ok());
    }
}
