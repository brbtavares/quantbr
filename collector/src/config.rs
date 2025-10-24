use clap::{Parser, ValueHint};

/// Collector CLI; connects to Binance public streams and writes raw ndjson.gz files.
#[derive(Debug, Parser, Clone)]
#[command(name = "collector", version, about = "Binance collector -> ndjson.gz")] 
pub struct Cli {
    /// Symbol (e.g., BTCUSDT)
    #[arg(short = 's', long = "symbol", default_value = "BTCUSDT")]
    pub symbol: String,
    /// Output raw data root directory
    #[arg(short = 'd', long = "dir", default_value = "data/raw", value_hint = ValueHint::DirPath)]
    pub raw_dir: String,
    /// Heartbeat interval in seconds
    #[arg(long = "heartbeat", default_value_t = 15)]
    pub heartbeat_sec: u64,
}
