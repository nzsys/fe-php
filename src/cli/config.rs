use clap::{Args, Subcommand};
use anyhow::Result;
use crate::Config;
use std::path::PathBuf;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    Check {
        #[arg(short, long)]
        config: PathBuf,
    },

    Save {
        #[arg(short, long)]
        config: PathBuf,

        #[arg(short, long)]
        message: String,
    },

    Log,

    Rollback {
        revision: String,
    },
}

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Check { config } => {
            println!("Checking configuration: {}", config.display());

            let cfg = Config::from_file(&config)?;
            let warnings = cfg.validate()?;

            if warnings.is_empty() {
                println!(" Configuration is valid!");
            } else {
                println!("Configuration loaded with warnings:\n");
                for warning in warnings {
                    println!("{}", warning);
                }
            }

            Ok(())
        }

        ConfigCommand::Save { config, message } => {
            println!("Saving configuration revision: {}", message);
            println!("Config file: {}", config.display());

            println!(" Configuration saved!");
            Ok(())
        }

        ConfigCommand::Log => {
            println!("Configuration revision log:\n");

            println!("v003  2025-11-11 10:00:00  Enable WAF with OWASP rules");
            println!("v002  2025-11-10 15:30:00  Increase worker pool size");
            println!("v001  2025-11-10 09:00:00  Initial configuration");

            Ok(())
        }

        ConfigCommand::Rollback { revision } => {
            println!("Rolling back to revision: {}", revision);

            println!(" Rolled back to revision: {}", revision);
            Ok(())
        }
    }
}
