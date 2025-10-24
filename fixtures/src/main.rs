use clap::{Parser, ValueHint};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tracing::info;

use datastore::rawparquet::RawParquetWriter;

#[derive(Debug, Parser, Clone)]
#[command(name = "fixtures", version, about = "Generate synthetic Parquet raw trades for a day")]
pub struct Cli {
    #[arg(short='s', long="symbol", default_value = "BTCUSDT")]
    pub symbol: String,
    /// Raw dir root
    #[arg(short='d', long="dir", default_value = "data/raw", value_hint = ValueHint::DirPath)]
    pub dir: String,
    /// Date YYYY-MM-DD
    #[arg(short='D', long="date", default_value = "2025-01-01")]
    pub date: String,
    /// Number of trades to simulate
    #[arg(short='n', long="num", default_value_t = 10_000)]
    pub num: usize,
    /// Seed for determinism
    #[arg(long="seed", default_value_t = 42)]
    pub seed: u64,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    let mut w = RawParquetWriter::new(&cli.dir, &cli.symbol)?;

    // Build trades uniformly over the day starting at midnight UTC
    let base_ms = chrono::NaiveDate::parse_from_str(&cli.date, "%Y-%m-%d")?
        .and_hms_opt(0,0,0).unwrap().and_utc().timestamp_millis();

    let mut rng = StdRng::seed_from_u64(cli.seed);
    let mut price: f64 = 50_000.0;
    for i in 0..cli.num {
        let ts = base_ms + (i as i64 % 86_400) * 1000 / 4; // spread across the day
        let dp = rng.gen_range(-5.0..5.0);
    price = (price + dp).max(100.0_f64);
        let qty = rng.gen_range(0.001..0.5);
        let is_buyer_maker = rng.gen_bool(0.5);
        let tr = common::Trade{ ts, seq: Some(i as i64), price, qty, is_buyer_maker, symbol: cli.symbol.clone() };
        w.append(tr)?;
    }
    info!(count = cli.num, "wrote synthetic trades");
    Ok(())
}
