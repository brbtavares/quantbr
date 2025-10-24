//! common: shared types, time utils, config, error types.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub type TsMs = i64; // milliseconds since epoch
pub type TsNs = i64; // nanoseconds since epoch

/// Represents a trade from Binance public streams.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trade {
    /// Event time (ms since epoch) as provided by exchange
    pub ts: TsMs,
    /// Unique (best-effort) sequence id per stream if available
    pub seq: Option<i64>,
    pub price: f64,
    pub qty: f64,
    /// true if buyer is market maker (Binance's isBuyerMarketMaker), we invert to buy/sell side
    pub is_buyer_maker: bool,
    pub symbol: String,
}

/// 1-second OHLCV bar with additional derived fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bar1s {
    pub ts_ns: TsNs,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub order_imbalance: f64,
}

/// Feature row extends Bar1s with technical indicators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureRow {
    pub ts_ns: TsNs,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub order_imbalance: f64,
    pub atr20: f64,
    pub ema20: f64,
}

/// Time and parsing utilities.
pub mod timeutils {
    use super::*;
    /// Convert milliseconds since epoch to nanoseconds.
    pub fn ms_to_ns(ms: TsMs) -> TsNs { ms.saturating_mul(1_000_000) }

    /// Truncate ms timestamp to the start of its 1-second bucket as ns.
    pub fn sec_bucket_ns(ms: TsMs) -> TsNs { ms / 1000 * 1_000_000_000 }

    /// Parse an ISO date (YYYY-MM-DD) into NaiveDate.
    pub fn parse_date(s: &str) -> anyhow::Result<NaiveDate> {
        Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)
    }

    /// Convert RFC3339 to ms since epoch.
    pub fn rfc3339_to_ms(s: &str) -> anyhow::Result<TsMs> {
        let dt: DateTime<Utc> = s.parse()?;
        Ok(dt.timestamp_millis())
    }

    /// Build file date string YYYY-MM-DD from ms.
    pub fn ymd_from_ms(ms: TsMs) -> String {
        let dt = DateTime::<Utc>::from_timestamp_millis(ms)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp_millis(0).unwrap());
        dt.date_naive().to_string()
    }
}

/// Errors used across crates.
#[derive(thiserror::Error, Debug)]
pub enum QuantError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Other: {0}")]
    Other(String),
}

/// Configs shared across crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub symbol: String,
    pub raw_dir: String,
    /// Heartbeat interval in seconds
    pub heartbeat_sec: u64,
}
