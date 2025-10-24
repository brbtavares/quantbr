use clap::Parser;
use tracing_subscriber::EnvFilter;

use backtester::cli::Cli;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let cli = Cli::parse();
    backtester::engine::run(cli)
}
