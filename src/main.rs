use clap::{Parser, Subcommand};
use fe_php::cli;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "fe-php")]
#[command(version = fe_php::VERSION)]
#[command(about = "All-in-one PHP application platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP server
    Serve(cli::serve::ServeArgs),

    /// Run benchmark tests
    Bench(cli::bench::BenchArgs),

    /// Configuration management
    Config(cli::config::ConfigArgs),

    /// Run sandbox tests
    Sandbox(cli::sandbox::SandboxArgs),

    /// Compare configurations
    Compare(cli::compare::CompareArgs),

    /// WAF management
    Waf(cli::waf::WafArgs),

    /// Monitor server status (TUI/JSON/Text)
    Monitor(cli::monitor::MonitorArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args) => cli::serve::run(args).await,
        Commands::Bench(args) => cli::bench::run(args).await,
        Commands::Config(args) => cli::config::run(args).await,
        Commands::Sandbox(args) => cli::sandbox::run(args).await,
        Commands::Compare(args) => cli::compare::run(args).await,
        Commands::Waf(args) => cli::waf::run(args).await,
        Commands::Monitor(args) => cli::monitor::run(args).await,
    }
}
