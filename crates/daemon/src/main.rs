#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "daemon",
    version,
    about = "Layer 1 thin local runner bootstrap"
)]
struct Cli {
    /// Placeholder flag until the full Layer 1 command surface lands.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    if cli.dry_run {
        tracing::info!("daemon bootstrap dry run complete");
    } else {
        tracing::info!("daemon bootstrap initialized");
    }

    Ok(())
}
