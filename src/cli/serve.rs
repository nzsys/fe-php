use clap::Args;
use anyhow::Result;
use crate::{Config, Server};
use std::path::PathBuf;
use tracing::info;

#[derive(Args)]
pub struct ServeArgs {
    /// Path to configuration file
    #[arg(short, long, default_value = "fe-php.toml")]
    pub config: PathBuf,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    // Load configuration
    let config = Config::from_file(&args.config)?;

    // Initialize logging
    crate::logging::init_logging(&config.logging.level, &config.logging.format)?;

    info!("Starting fe-php server v{}", crate::VERSION);
    info!("Loading configuration from: {}", args.config.display());

    // Validate configuration
    let warnings = config.validate()?;
    for warning in warnings {
        println!("{}", warning);
    }

    // Setup signal handlers
    crate::utils::setup_signal_handlers().await?;

    // Initialize metrics
    crate::metrics::init_metrics();

    // Create and start server
    let server = Server::new(config).await?;
    info!("Server starting...");

    server.serve().await?;

    Ok(())
}
