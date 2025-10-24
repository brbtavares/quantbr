use clap::{Parser, ValueHint};

/// Build 1s features from raw Parquet trades.
#[derive(Debug, Parser, Clone)]
#[command(name = "feature_builder", version, about = "Aggregate trades -> 1s features Parquet")]
pub struct Cli {
    /// Symbol (e.g., BTCUSDT)
    #[arg(short = 's', long = "symbol")]
    pub symbol: String,
    /// Input path root (raw)
    #[arg(long = "input", default_value = "data/raw", value_hint = ValueHint::DirPath)]
    pub input: String,
    /// Output path root (features)
    #[arg(long = "output", default_value = "data/features", value_hint = ValueHint::DirPath)]
    pub output: String,
    /// From date (YYYY-MM-DD)
    #[arg(long = "from")]
    pub from: String,
    /// To date inclusive (YYYY-MM-DD)
    #[arg(long = "to")]
    pub to: String,
}
