use super::executor::{PhpExecutor, PhpRequest, PhpResponse};
use super::ffi::PhpFfi;
use super::PhpConfig;
use anyhow::Result;
use async_channel::{Sender, Receiver, bounded};
use std::sync::{Arc, Barrier};
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
    shared_ffi: Option<Arc<PhpFfi>>,   // Shared FFI instance for all workers
}

impl WorkerPool {
    pub fn new(php_config: PhpConfig, config: WorkerPoolConfig) -> Result<Self> {
        let (request_tx, request_rx) = bounded(config.pool_size * 2);

        // Initialize PHP module ONCE globally (not in worker threads)
        // This prevents "zend_mm_heap corrupted" error when multiple workers
        // try to call php_module_startup() simultaneously
        let (php_module, shared_ffi) = if !php_config.use_fpm {
            info!("Initializing PHP module for {} worker(s)...", config.pool_size);
            let module = PhpExecutor::new(php_config.clone())?;
            let ffi = module.get_shared_ffi();
            info!("PHP module initialized successfully");
            (Some(module), ffi)
        } else {
            (None, None)  // PHP-FPM mode doesn't need global initialization
        };

        // Create a barrier to synchronize worker thread initialization
        // This ensures all workers are fully initialized before accepting requests
        let barrier = Arc::new(Barrier::new(config.pool_size + 1));

        // Spawn worker threads
        for worker_id in 0..config.pool_size {
            let request_rx = request_rx.clone();
            let php_config = php_config.clone();
            let max_requests = config.max_requests;
            let shared_ffi = shared_ffi.clone();
            let barrier = Arc::clone(&barrier);

            task::spawn_blocking(move || {
                Self::worker_thread(worker_id, request_rx, php_config, max_requests, shared_ffi, barrier);
            });
        }

        // Wait for all workers to initialize
        info!("Waiting for {} workers to initialize...", config.pool_size);
        barrier.wait();
        info!("All PHP workers initialized and ready");

        Ok(Self {
            request_tx,
            config,
            _php_module: php_module,  // Kept alive for process lifetime
            shared_ffi,               // Kept alive and shared with all workers
        })
    }

    fn worker_thread(
        worker_id: usize,
        request_rx: Receiver<(PhpRequest, Sender<Result<PhpResponse>>)>,
        php_config: PhpConfig,
        max_requests: usize,
        shared_ffi: Option<Arc<PhpFfi>>,
        barrier: Arc<Barrier>,
    ) {
        info!("Worker {} starting initialization...", worker_id);

        // Initialize PHP executor for this worker
        // Use new_worker() with shared PhpFfi instance (no need to load library or call module_startup)
        let executor = match PhpExecutor::new_worker(php_config, shared_ffi) {
            Ok(exec) => {
                info!("Worker {} initialized successfully", worker_id);
                exec
            }
            Err(e) => {
                error!("Worker {} failed to initialize PHP: {}", worker_id, e);
                // Still wait at barrier to avoid deadlock
                barrier.wait();
                return;
            }
        };

        // Initialize TSRM thread-local resources for this worker thread (ZTS only)
        // This MUST be done before processing any PHP requests
        executor.thread_init();

        // Wait for all workers to initialize before processing requests
        // This prevents race conditions during startup
        barrier.wait();
        info!("Worker {} ready to accept requests", worker_id);

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

        // Free TSRM thread-local resources before thread exits (ZTS only)
        executor.thread_cleanup();

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
            use_fpm: false,
            fpm_socket: String::from("127.0.0.1:9000"),
        };

        let pool_config = WorkerPoolConfig {
            pool_size: 2,
            max_requests: 1000,
        };

        let result = WorkerPool::new(php_config, pool_config);
        assert!(result.is_ok());
    }
}
