use clap::Parser;
use tracing_subscriber::EnvFilter;

use feature_builder::cli::Cli;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let cli = Cli::parse();
    feature_builder::transforms::run(cli)
}
