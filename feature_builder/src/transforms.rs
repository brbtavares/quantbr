use anyhow::Context;
use common::{timeutils, Bar1s, FeatureRow, Trade};
use tracing::info;

use crate::cli::Cli;
use datastore::{rawparquet, parquetio, manifest};

/// Run feature building for date range.
pub fn run(cli: Cli) -> anyhow::Result<()> {
    let from = timeutils::parse_date(&cli.from)?;
    let to = timeutils::parse_date(&cli.to)?;

    let mut date = from;
    let mut written_dates: Vec<String> = Vec::new();
    while date <= to {
        let ds = date.to_string();
        let trades = rawparquet::read_raw_day(&cli.input, &cli.symbol, &ds)
            .with_context(|| format!("read raw {ds}"))?;
        let bars = aggregate_1s(&trades);
        let feats = compute_features(&bars);
        parquetio::write_features(&cli.output, &cli.symbol, &ds, &feats)?;
        info!(%ds, count=?feats.len(), "wrote features");
        written_dates.push(ds.clone());
        date = date.succ_opt().unwrap();
    }
    if !written_dates.is_empty() {
        manifest::write(&cli.output, &cli.symbol, &written_dates)?;
    }
    Ok(())
}

// no parse_trades needed; raw parquet provides Trade rows directly

/// Aggregate trades into 1-second bars.
pub fn aggregate_1s(trades: &[Trade]) -> Vec<Bar1s> {
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<i64, Vec<&Trade>> = BTreeMap::new();
    for t in trades { buckets.entry(t.ts / 1000).or_default().push(t); }

    let mut out = Vec::with_capacity(buckets.len());
    for (sec, ts) in buckets.into_iter() {
        if ts.is_empty() { continue; }
        let open = ts.first().unwrap().price;
        let close = ts.last().unwrap().price;
        let high = ts.iter().map(|t| t.price).fold(f64::MIN, f64::max);
        let low = ts.iter().map(|t| t.price).fold(f64::MAX, f64::min);
        let volume: f64 = ts.iter().map(|t| t.qty).sum();
        let vwap = if volume > 0.0 { ts.iter().map(|t| t.price * t.qty).sum::<f64>() / volume } else { close };
        let buy_volume: f64 = ts.iter().filter(|t| !t.is_buyer_maker).map(|t| t.qty).sum();
        let sell_volume: f64 = ts.iter().filter(|t| t.is_buyer_maker).map(|t| t.qty).sum();
        let order_imbalance = if volume > 0.0 { (buy_volume - sell_volume) / volume } else { 0.0 };
        out.push(Bar1s{
            ts_ns: sec * 1_000_000_000,
            open, high, low, close, volume, vwap, buy_volume, sell_volume, order_imbalance
        });
    }
    out
}

/// Compute ATR20 and EMA20 on close.
pub fn compute_features(bars: &[Bar1s]) -> Vec<FeatureRow> {
    let mut ema = 0.0;
    let alpha = 2.0 / (20.0 + 1.0);
    let mut prev_close = None;
    let mut atrs: Vec<f64> = Vec::with_capacity(bars.len());
    let mut feats = Vec::with_capacity(bars.len());

    for (i, b) in bars.iter().enumerate() {
        // true range
        let prev_c = prev_close.unwrap_or(b.close);
        let tr = (b.high - b.low)
            .abs()
            .max((b.high - prev_c).abs())
            .max((b.low - prev_c).abs());
        atrs.push(tr);
        let atr20 = if i+1 < 20 { f64::NAN } else { let s: f64 = atrs[i+1-20..=i].iter().sum(); s/20.0 };

        if i == 0 { ema = b.close; } else { ema = alpha * b.close + (1.0 - alpha) * ema; }
        prev_close = Some(b.close);

        feats.push(FeatureRow{
            ts_ns: b.ts_ns,
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
            vwap: b.vwap,
            buy_volume: b.buy_volume,
            sell_volume: b.sell_volume,
            order_imbalance: b.order_imbalance,
            atr20,
            ema20: ema,
        });
    }
    feats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema20_basic() {
        let mut bars = Vec::new();
        for i in 0..30 { bars.push(Bar1s{ ts_ns: i, open: 1.0, high: 1.0, low: 1.0, close: i as f64, volume: 1.0, vwap: i as f64, buy_volume: 0.5, sell_volume: 0.5, order_imbalance: 0.0 }); }
        let feats = compute_features(&bars);
        assert_eq!(feats.len(), 30);
        // First EMA equals first close; here it's 0.0
        assert_eq!(feats[0].ema20, bars[0].close);
        assert!(feats.last().unwrap().ema20 > feats[10].ema20);
    }

    #[test]
    fn test_aggregate_1s_vwap() {
        let mut trades = Vec::new();
        trades.push(Trade{ ts: 1000, seq: Some(1), price: 10.0, qty: 1.0, is_buyer_maker: false, symbol: "X".into() });
        trades.push(Trade{ ts: 1100, seq: Some(2), price: 20.0, qty: 2.0, is_buyer_maker: true, symbol: "X".into() });
        let bars = aggregate_1s(&trades);
        assert_eq!(bars.len(), 1);
        assert!((bars[0].vwap - (10.0*1.0 + 20.0*2.0)/3.0).abs() < 1e-9);
        assert!(bars[0].buy_volume > 0.0 && bars[0].sell_volume > 0.0);
    }
}
