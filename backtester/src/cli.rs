use clap::{Parser, ValueHint};

/// Backtest a simple momentum strategy using feature Parquet files.
#[derive(Debug, Parser, Clone)]
#[command(name = "backtester", version, about = "Simple momentum backtester")] 
pub struct Cli {
    /// Symbol (e.g., BTCUSDT)
    #[arg(short = 's', long = "symbol")]
    pub symbol: String,
    /// Features dir root
    #[arg(long = "features", default_value = "data/features", value_hint = ValueHint::DirPath)]
    pub features: String,
    /// From date YYYY-MM-DD
    #[arg(long = "from")]
    pub from: String,
    /// To date YYYY-MM-DD
    #[arg(long = "to")]
    pub to: String,
    /// Report CSV path
    #[arg(long = "report", default_value = "reports/report.csv", value_hint = ValueHint::FilePath)]
    pub report: String,
    /// Sigma threshold multiplier
    #[arg(long = "sigma", default_value_t = 1.5)]
    pub sigma: f64,
    /// Fee rate per trade (e.g., 0.0004)
    #[arg(long = "fee", default_value_t = 0.0004)]
    pub fee: f64,
    /// Slippage in ticks percentage (applied on fill)
    #[arg(long = "slippage", default_value_t = 0.0005)]
    pub slippage: f64,
}
