use std::{fs, path::Path};

use anyhow::Context;
use polars::prelude::*;
use tracing::info;

use crate::{cli::Cli, metrics};
use common::timeutils;
use datastore::parquetio;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Pos { Flat, Long, Short }

impl Default for Pos {
    fn default() -> Self { Pos::Flat }
}

#[derive(Debug, Default)]
struct Ledger {
    equity: f64,
    cash: f64,
    pos: Pos,
    entry_price: f64,
    max_equity: f64,
    returns: Vec<f64>,
}

impl Ledger {
    fn new() -> Self { Self{ equity: 0.0, cash: 0.0, pos: Pos::Flat, entry_price: 0.0, max_equity: 0.0, returns: vec![] } }
    fn mark(&mut self, price: f64) {
        let pnl = match self.pos {
            Pos::Flat => 0.0,
            Pos::Long => price - self.entry_price,
            Pos::Short => self.entry_price - price,
        };
        self.equity = self.cash + pnl;
        self.max_equity = self.max_equity.max(self.equity);
        self.returns.push(self.equity);
    }
    fn enter(&mut self, price: f64, pos: Pos, fee: f64, slippage: f64) {
        let fill = price * (1.0 + if pos==Pos::Long { slippage } else { -slippage });
        self.entry_price = fill * (1.0 + fee);
        self.pos = pos;
    }
    fn exit(&mut self, price: f64, fee: f64, slippage: f64) {
        if self.pos == Pos::Flat { return; }
        let sign = if self.pos==Pos::Long { 1.0 } else { -1.0 };
        let fill = price * (1.0 - sign*slippage);
        let pnl = sign * (fill * (1.0 - fee) - self.entry_price);
        self.cash += pnl;
        self.pos = Pos::Flat;
        self.entry_price = 0.0;
    }
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    fs::create_dir_all(Path::new(&cli.report).parent().unwrap_or(Path::new(".")))?;

    let from = timeutils::parse_date(&cli.from)?;
    let to = timeutils::parse_date(&cli.to)?;

    let mut date = from;
    let mut df_all: Option<DataFrame> = None;
    while date <= to {
        let ds = date.to_string();
        let df = parquetio::read_features(&cli.features, &cli.symbol, &ds)
            .with_context(|| format!("read features {ds}"))?;
    df_all = Some(match df_all { None => df, Some(acc) => { acc.vstack(&df)? } });
        date = date.succ_opt().unwrap();
    }

    let df = df_all.context("no data")?;
    // compute rolling std of (close - ema20) as sigma baseline using simple rolling over 60s
    let spread = df.column("close")?.f64()?.into_no_null_iter()
        .zip(df.column("ema20")?.f64()?.into_no_null_iter())
        .map(|(c, e)| c - e).collect::<Vec<f64>>();
    let sigma = rolling_std(&spread, 60);

    let mut ledger = Ledger::new();
    for i in 0..df.height() {
        let close = df.column("close")?.f64()?.get(i).unwrap();
        let ema = df.column("ema20")?.f64()?.get(i).unwrap();
        ledger.mark(close);
        let sig = sigma[i];
        if sig.is_finite() && sig>0.0 {
            let z = (close - ema) / sig;
            if z > cli.sigma {
                if ledger.pos != Pos::Long { ledger.exit(close, cli.fee, cli.slippage); ledger.enter(close, Pos::Long, cli.fee, cli.slippage); }
            } else if z < -cli.sigma {
                if ledger.pos != Pos::Short { ledger.exit(close, cli.fee, cli.slippage); ledger.enter(close, Pos::Short, cli.fee, cli.slippage); }
            }
        }
    }
    // close at end
    if df.height()>0 { let last = df.column("close")?.f64()?.get(df.height()-1).unwrap(); ledger.exit(last, cli.fee, cli.slippage); ledger.mark(last); }

    let report = metrics::report(&ledger.returns);
    write_report_csv(&cli.report, &report)?;
    info!(?report, "backtest done");
    Ok(())
}

fn rolling_std(vals: &[f64], win: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(vals.len());
    let mut sum = 0.0;
    let mut sum2 = 0.0;
    let mut q: std::collections::VecDeque<f64> = std::collections::VecDeque::new();
    for &v in vals {
        q.push_back(v);
        sum += v; sum2 += v*v;
        if q.len()>win { let old = q.pop_front().unwrap(); sum -= old; sum2 -= old*old; }
        if q.len()<win { out.push(f64::NAN); } else { let n=win as f64; let mean=sum/n; let var=(sum2/n - mean*mean).max(0.0); out.push(var.sqrt()); }
    }
    out
}

fn write_report_csv(path: &str, r: &metrics::Report) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.serialize(r)?;
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_std() {
        let v = vec![1.0,2.0,3.0,4.0,5.0];
        let s = rolling_std(&v, 3);
        assert!(s[0].is_nan() && s[1].is_nan());
        assert!(s[2] > 0.0);
    }
}
