pub mod config;
pub mod server;
pub mod php;
pub mod waf;
pub mod metrics;
pub mod logging;
pub mod admin;
pub mod cli;
pub mod utils;

// Phase 5 & 6 Advanced Features
pub mod tls;
pub mod geoip;
pub mod redis_session;
pub mod tracing_telemetry;
pub mod load_balancing;
pub mod deployment;

pub use config::Config;
pub use server::Server;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
