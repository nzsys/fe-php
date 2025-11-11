use anyhow::Result;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use futures::stream::StreamExt;
use tracing::{info, warn};

pub async fn setup_signal_handlers() -> Result<()> {
    let signals = Signals::new(&[SIGTERM, SIGINT, SIGUSR1, SIGUSR2])?;
    let mut signals = signals.fuse();

    tokio::spawn(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    info!("Received shutdown signal, gracefully shutting down...");
                    std::process::exit(0);
                }
                SIGUSR1 => {
                    info!("Received USR1 signal - Graceful reload (not implemented yet)");
                    // TODO: Implement configuration reload
                }
                SIGUSR2 => {
                    info!("Received USR2 signal - Maintenance mode toggle (not implemented yet)");
                    // TODO: Implement maintenance mode
                }
                _ => {
                    warn!("Received unknown signal: {}", signal);
                }
            }
        }
    });

    Ok(())
}
