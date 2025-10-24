use clap::Parser;
use tracing_subscriber::EnvFilter;

use collector::config::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    collector::ws_client::run(cli).await
}
