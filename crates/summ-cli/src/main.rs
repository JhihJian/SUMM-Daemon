use anyhow::Result;
use clap::Parser;
use commands::Commands;

mod client;
mod commands;

/// SUMM CLI - Client for SUMM Daemon process management service
#[derive(Parser, Debug)]
#[command(name = "summ")]
#[command(author = "SUMM Team")]
#[command(version = "0.1.0")]
#[command(about = "CLI client for SUMM Daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    cli.command.execute().await
}
