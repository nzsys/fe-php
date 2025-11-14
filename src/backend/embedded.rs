use super::{Backend, BackendError, BackendType, HealthStatus};
use crate::php::{WorkerPool, PhpRequest, PhpResponse};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

pub struct EmbeddedBackend {
    worker_pool: Arc<WorkerPool>,
}

impl EmbeddedBackend {
    pub fn new(worker_pool: Arc<WorkerPool>) -> Self {
        Self { worker_pool }
    }
}

impl Backend for EmbeddedBackend {
    fn execute(&self, request: PhpRequest) -> Result<PhpResponse, BackendError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                self.worker_pool.execute(request)
            )
        })
        .map_err(|e| BackendError::PhpError(e.to_string()))
    }

    fn health_check(&self) -> Result<HealthStatus> {
        let start = Instant::now();

        let healthy = self.worker_pool.executor().is_some();
        let latency = start.elapsed();

        if healthy {
            Ok(HealthStatus::healthy("Embedded backend is healthy")
                .with_latency(latency))
        } else {
            Ok(HealthStatus::unhealthy("Embedded backend has no executor (PHP-FPM mode?)"))
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Embedded
    }
}
