pub mod ffi;
pub mod worker;
pub mod executor;
pub mod fastcgi;
pub mod connection_pool;

pub use worker::{WorkerPool, WorkerPoolConfig};
pub use executor::{PhpExecutor, PhpRequest, PhpResponse};

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PhpConfig {
    pub libphp_path: PathBuf,
    pub document_root: PathBuf,
    pub worker_pool_size: usize,
    pub worker_max_requests: usize,
    pub use_fpm: bool,
    pub fpm_socket: String,
}

impl PhpConfig {
    pub fn new(
        libphp_path: PathBuf,
        document_root: PathBuf,
        worker_pool_size: usize,
        worker_max_requests: usize,
        use_fpm: bool,
        fpm_socket: String,
    ) -> Self {
        Self {
            libphp_path,
            document_root,
            worker_pool_size,
            worker_max_requests,
            use_fpm,
            fpm_socket,
        }
    }
}
