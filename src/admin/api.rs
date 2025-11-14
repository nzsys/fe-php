use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: usize,
    pub status: String,
    pub requests_handled: usize,
    pub memory_mb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub uptime_seconds: u64,
    pub active_connections: usize,
    pub total_requests: u64,
    pub workers: Vec<WorkerStatus>,
}

pub struct AdminApi {

}

impl AdminApi {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_status(&self) -> ServerStatus {
        ServerStatus {
            uptime_seconds: 0,
            active_connections: 0,
            total_requests: 0,
            workers: vec![],
        }
    }

    pub fn reload_config(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn restart_workers(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Default for AdminApi {
    fn default() -> Self {
        Self::new()
    }
}
