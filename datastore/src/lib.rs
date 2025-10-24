//! datastore: ndjson.gz and Parquet helpers using Polars
use std::{fs::{self, File}, io::{BufReader, Read, Write}, path::{Path, PathBuf}};

use flate2::{write::GzEncoder, Compression, bufread::GzDecoder};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

use common::timeutils;

pub mod ndjson {
    use super::*;

    /// Wrapper for raw message stored per line, gzipped.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RawMessage { pub ts: i64, pub seq: Option<i64>, pub payload: String }

    // Sanitize a string to be safely used as a single path component across platforms.
    // - Replace path separators and disallowed chars with '_'
    // - Keep ASCII alphanumerics, '-', '_', and '.'
    pub(crate) fn sanitize_component(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            let ok = c.is_ascii_alphanumeric() || c == '-' || c == '_';
            if ok {
                out.push(c);
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }
        // Avoid problematic trailing spaces or dots on Windows
        while out.ends_with(' ') || out.ends_with('.') { out.pop(); }
        // Also trim trailing underscores that may result from replacing trailing punctuation
        while out.ends_with('_') { out.pop(); }
        if out.is_empty() { "_".into() } else { out }
    }

    fn raw_path(root: &str, symbol: &str, date: &str) -> PathBuf {
        let sym = sanitize_component(symbol);
        Path::new(root).join(sym).join(format!("{date}.ndjson.gz"))
    }

    /// Append-only writer that rotates by day.
    pub struct GzNdjsonWriter {
        root: String,
        symbol: String,
        current_date: String,
        encoder: GzEncoder<File>,
    }

    impl GzNdjsonWriter {
        pub fn new(root: &str, symbol: &str) -> anyhow::Result<Self> {
            let sym = sanitize_component(symbol);
            fs::create_dir_all(Path::new(root).join(&sym))?;
            let today = chrono::Utc::now().date_naive().to_string();
            let path = raw_path(root, &sym, &today);
            let file = File::options().create(true).append(true).open(&path)?;
            let encoder = GzEncoder::new(file, Compression::default());
            Ok(Self{ root: root.into(), symbol: symbol.into(), current_date: today, encoder })
        }
        pub fn append(&mut self, msg: &RawMessage) -> anyhow::Result<()> {
            let date = timeutils::ymd_from_ms(msg.ts);
            if date != self.current_date {
                self.rotate(&date)?;
            }
            let line = serde_json::to_string(msg)? + "\n";
            self.encoder.write_all(line.as_bytes())?;
            self.encoder.flush()?;
            Ok(())
        }
        fn rotate(&mut self, date: &str) -> anyhow::Result<()> {
            // finish old encoder by dropping it
            let root = self.root.clone();
            let symbol = self.symbol.clone();
            // Reopen new file
            let path = raw_path(&root, &symbol, date);
            let file = File::options().create(true).append(true).open(&path)?;
            self.encoder = GzEncoder::new(file, Compression::default());
            self.current_date = date.to_string();
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_sanitize_component() {
            assert_eq!(sanitize_component("BTCUSDT"), "BTCUSDT");
            assert_eq!(sanitize_component("BTC/USDT"), "BTC_USDT");
            assert_eq!(sanitize_component("ETH:USDT"), "ETH_USDT");
            assert_eq!(sanitize_component("SOL USDT."), "SOL_USDT");
            // collapse multiple invalids into single underscore
            assert_eq!(sanitize_component("../weird\\name"), "_weird_name");
        }
    }

    /// Read all RawMessage from a given date file.
    pub fn read_raw_day(root: &str, symbol: &str, date: &str) -> anyhow::Result<Vec<RawMessage>> {
        let path = raw_path(root, symbol, date);
        let file = File::open(path)?;
        let mut decoder = GzDecoder::new(BufReader::new(file));
        let mut buf = String::new();
        decoder.read_to_string(&mut buf)?;
        let mut out = Vec::new();
        for line in buf.lines() {
            if line.trim().is_empty() { continue; }
            out.push(serde_json::from_str(line)?);
        }
        Ok(out)
    }
}

pub mod parquetio {
    use super::*;

    fn feat_path(root: &str, symbol: &str, date: &str) -> PathBuf {
        // reuse sanitizer from ndjson module
        let sym = ndjson::sanitize_component(symbol);
        Path::new(root).join(sym).join(format!("{date}.parquet"))
    }

    /// Write 1s features as Parquet via Polars DataFrame.
    pub fn write_features(root: &str, symbol: &str, date: &str, rows: &[common::FeatureRow]) -> anyhow::Result<()> {
        let sym = ndjson::sanitize_component(symbol);
        fs::create_dir_all(Path::new(root).join(&sym))?;
        let path = feat_path(root, symbol, date);
        let df = feature_rows_to_df(rows)?;
        ParquetWriter::new(File::create(path)?).finish(&mut df.clone())?;
        Ok(())
    }

    /// Read feature Parquet into DataFrame.
    pub fn read_features(root: &str, symbol: &str, date: &str) -> anyhow::Result<DataFrame> {
        let path = feat_path(root, symbol, date);
        Ok(ParquetReader::new(File::open(path)?).finish()?)
    }

    fn feature_rows_to_df(rows: &[common::FeatureRow]) -> anyhow::Result<DataFrame> {
        let ts: Vec<i64> = rows.iter().map(|r| r.ts_ns).collect();
        macro_rules! col { ($name:ident) => { rows.iter().map(|r| r.$name).collect::<Vec<f64>>() } }
        let df = df!(
            "ts_ns" => ts,
            "open" => col!(open),
            "high" => col!(high),
            "low" => col!(low),
            "close" => col!(close),
            "volume" => col!(volume),
            "vwap" => col!(vwap),
            "buy_volume" => col!(buy_volume),
            "sell_volume" => col!(sell_volume),
            "order_imbalance" => col!(order_imbalance),
            "atr20" => col!(atr20),
            "ema20" => col!(ema20)
        )?;
        Ok(df)
    }
}

/// A tiny manifest structure listing available dates for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest { pub symbol: String, pub dates: Vec<String> }

pub mod manifest {
    use super::*;

    fn manifest_path(root: &str, symbol: &str) -> PathBuf {
        let sym = ndjson::sanitize_component(symbol);
        Path::new(root).join(sym).join("_manifest.json")
    }
    pub fn write(root: &str, symbol: &str, dates: &[String]) -> anyhow::Result<()> {
        let sym = ndjson::sanitize_component(symbol);
        fs::create_dir_all(Path::new(root).join(&sym))?;
        let m = Manifest{ symbol: symbol.to_string(), dates: dates.to_vec() };
        let path = manifest_path(root, symbol);
        fs::write(path, serde_json::to_vec_pretty(&m)?)?;
        Ok(())
    }
    pub fn read(root: &str, symbol: &str) -> anyhow::Result<Manifest> {
        let path = manifest_path(root, symbol);
        let data = fs::read(path)?;
        Ok(serde_json::from_slice(&data)?)
    }
}
