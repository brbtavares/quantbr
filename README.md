# quantbr

Quick design summary:
- End-to-end crypto quant pipeline: collector -> raw storage -> feature builder -> backtester.
- Clear workspace layout with shared `common` crate and I/O helpers in `datastore`.
- Deterministic processing: 1s bars, ordered events, seeded tests.
- Minimal but realistic: reconnection/backoff, EMA/VWAP/ATR features, simple momentum strategy.
- CI runs build, tests, and an E2E smoke test using generated fixtures.

## Quickstart

Prereqs: Rust stable toolchain.

Run the full pipeline on synthetic BTCUSDT data:

```pwsh
# 1) Generate synthetic raw trades for one day
cargo run -p fixtures -- -s BTCUSDT -d .\data\raw -D 2025-01-01 -n 20000

# 2) Build 1s features to Parquet
cargo run -p feature_builder -- -s BTCUSDT --input .\data\raw --output .\data\features --from 2025-01-01 --to 2025-01-01

# 3) Backtest simple momentum strategy and write report CSV
cargo run -p backtester -- -s BTCUSDT --features .\data\features --from 2025-01-01 --to 2025-01-01 --report .\reports\bt_BTCUSDT_2025-01-01.csv
```

Notes:
- Collector connects to Binance public streams (spot trades). For CI and local tests we use fixtures.
- Paths follow layout:
  - raw trades (Parquet):
    - single file per day: `data/raw/{symbol}/YYYY-MM-DD.parquet`, or
    - sharded parts per day: `data/raw/{symbol}/YYYY-MM-DD/part-*.parquet`
  - features (Parquet): `data/features/{symbol}/YYYY-MM-DD.parquet`
  - reports: `reports/*.csv`

## Crates
- `common`: shared types, time utils, config, error types.
- `datastore`: helpers for Parquet I/O with Polars (raw trades and features).
- `collector`: Binance connector; writes raw Parquet trades.
- `feature_builder`: aggregates trades to 1s bars and computes features.
- `backtester`: event engine for momentum strategy; outputs metrics CSV.
- `fixtures`: generates synthetic trade fixtures for tests and smoke run.

## Dev

```pwsh
cargo build --workspace
cargo test --workspace
# Optional: lint
cargo clippy --all-targets -- -D warnings
```

### Commit style
We recommend Conventional Commits. You can use commitizen locally if you like.

