//! datastore: Parquet helpers using Polars
use std::{fs::{self, File}, path::{Path, PathBuf}};

use polars::prelude::*;
use serde::{Deserialize, Serialize};

use common::timeutils;

/// Sanitize a string to be safely used as a single path component across platforms.
/// - Replace path separators and disallowed chars with '_'
/// - Keep ASCII alphanumerics, '-', and '_'
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

pub mod parquetio {
    use super::*;

    fn feat_path(root: &str, symbol: &str, date: &str) -> PathBuf {
        let sym = sanitize_component(symbol);
        Path::new(root).join(sym).join(format!("{date}.parquet"))
    }

    /// Write 1s features as Parquet via Polars DataFrame.
    pub fn write_features(root: &str, symbol: &str, date: &str, rows: &[common::FeatureRow]) -> anyhow::Result<()> {
        let sym = sanitize_component(symbol);
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

/// Raw Parquet I/O for trades.
pub mod rawparquet {
    use super::*;

    fn date_dir(root: &str, symbol: &str, date: &str) -> PathBuf {
        let sym = sanitize_component(symbol);
        Path::new(root).join(sym).join(date)
    }

    fn single_path(root: &str, symbol: &str, date: &str) -> PathBuf {
        let sym = sanitize_component(symbol);
        Path::new(root).join(sym).join(format!("{date}.parquet"))
    }

    fn part_path(dir: &Path, ts_ms: i64, part_idx: usize) -> PathBuf {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts_ms)
            .unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp_millis(0).unwrap());
        let label = dt.format("%H%M%S");
        dir.join(format!("part-{}_{}.parquet", label, part_idx))
    }

    /// Buffered writer that flushes trades into Parquet part files under <root>/<symbol>/<date>/.
    pub struct RawParquetWriter {
        root: String,
        symbol: String,
        buf: Vec<common::Trade>,
        flush_every: usize,
        part_idx: usize,
        current_date: String,
    }

    impl RawParquetWriter {
        pub fn new(root: &str, symbol: &str) -> anyhow::Result<Self> {
            // Initialize with today's date; will rotate if date changes across appends
            let today = chrono::Utc::now().date_naive().to_string();
            let sym = sanitize_component(symbol);
            fs::create_dir_all(Path::new(root).join(&sym).join(&today))?;
            Ok(Self{ root: root.into(), symbol: symbol.into(), buf: Vec::with_capacity(10_000), flush_every: 10_000, part_idx: 0, current_date: today })
        }

        pub fn append(&mut self, t: common::Trade) -> anyhow::Result<()> {
            // rotate date dir if needed
            let d = timeutils::ymd_from_ms(t.ts);
            if d != self.current_date {
                self.flush()?;
                self.current_date = d.clone();
                let sym = sanitize_component(&self.symbol);
                fs::create_dir_all(Path::new(&self.root).join(sym).join(&d))?;
                self.part_idx = 0;
            }
            self.buf.push(t);
            if self.buf.len() >= self.flush_every { self.flush()?; }
            Ok(())
        }

        fn flush(&mut self) -> anyhow::Result<()> {
            if self.buf.is_empty() { return Ok(()); }
            let date_dir = date_dir(&self.root, &self.symbol, &self.current_date);
            fs::create_dir_all(&date_dir)?;
            // Use the first row ts to name the part for determinism
            let ts0 = self.buf.first().map(|t| t.ts).unwrap_or(0);
            let path = part_path(&date_dir, ts0, self.part_idx);
            self.part_idx += 1;
            let df = trades_to_df(&self.buf)?;
            ParquetWriter::new(File::create(path)?).finish(&mut df.clone())?;
            self.buf.clear();
            Ok(())
        }
    }

    impl Drop for RawParquetWriter {
        fn drop(&mut self) {
            let _ = self.flush();
        }
    }

    fn trades_to_df(rows: &[common::Trade]) -> anyhow::Result<DataFrame> {
        let ts: Vec<i64> = rows.iter().map(|r| r.ts).collect();
        let seq: Vec<Option<i64>> = rows.iter().map(|r| r.seq).collect();
        let price: Vec<f64> = rows.iter().map(|r| r.price).collect();
        let qty: Vec<f64> = rows.iter().map(|r| r.qty).collect();
        let is_buyer_maker: Vec<bool> = rows.iter().map(|r| r.is_buyer_maker).collect();
        let symbol: Vec<String> = rows.iter().map(|r| r.symbol.clone()).collect();
        let df = df!(
            "ts" => ts,
            "seq" => seq,
            "price" => price,
            "qty" => qty,
            "is_buyer_maker" => is_buyer_maker,
            "symbol" => symbol
        )?;
        Ok(df)
    }

    fn df_to_trades(df: &DataFrame) -> anyhow::Result<Vec<common::Trade>> {
        let ts = df.column("ts")?.i64()?;
        let seq = df.column("seq")?.i64()?.clone(); // allow nulls
        let price = df.column("price")?.f64()?;
        let qty = df.column("qty")?.f64()?;
        let ibm = df.column("is_buyer_maker")?.bool()?;
        let sym_col = df.column("symbol")?;

        let mut out = Vec::with_capacity(df.height());
        for i in 0..df.height() {
            // Extract symbol as string without relying on utf8()-specific APIs
            let sym_str = match sym_col.get(i) {
                Ok(v) => v.to_string(),
                Err(_) => String::new(),
            };
            out.push(common::Trade{
                ts: ts.get(i).unwrap(),
                seq: seq.get(i),
                price: price.get(i).unwrap(),
                qty: qty.get(i).unwrap(),
                is_buyer_maker: ibm.get(i).unwrap(),
                symbol: sym_str,
            });
        }
        // order by (ts, seq)
        out.sort_by_key(|t| (t.ts, t.seq.unwrap_or(0)));
        Ok(out)
    }

    /// Read all trades for a day from a single parquet file or from all part-*.parquet under date dir.
    pub fn read_raw_day(root: &str, symbol: &str, date: &str) -> anyhow::Result<Vec<common::Trade>> {
        let single = single_path(root, symbol, date);
        if single.exists() {
            let df = ParquetReader::new(File::open(single)?).finish()?;
            return df_to_trades(&df);
        }
        let dir = date_dir(root, symbol, date);
        if dir.is_dir() {
            let mut files: Vec<PathBuf> = Vec::new();
            for e in fs::read_dir(&dir)? { let p = e?.path(); if p.extension().and_then(|e| e.to_str()) == Some("parquet") { files.push(p); } }
            files.sort();
            let mut all = Vec::new();
            for p in files {
                let df = ParquetReader::new(File::open(p)?).finish()?;
                all.extend(df_to_trades(&df)?);
            }
            return Ok(all);
        }
        anyhow::bail!("no raw parquet data found for {symbol} {date}")
    }
}

/// A tiny manifest structure listing available dates for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest { pub symbol: String, pub dates: Vec<String> }

pub mod manifest {
    use super::*;

    fn manifest_path(root: &str, symbol: &str) -> PathBuf {
        let sym = sanitize_component(symbol);
        Path::new(root).join(sym).join("_manifest.json")
    }
    pub fn write(root: &str, symbol: &str, dates: &[String]) -> anyhow::Result<()> {
        let sym = sanitize_component(symbol);
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
