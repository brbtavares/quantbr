# Examples

Run local pipeline on fixtures:

1. Generate data
   - `cargo run -p fixtures -- -s BTCUSDT -d ./data/raw -D 2025-01-01 -n 10000`
2. Build features
   - `cargo run -p feature_builder -- -s BTCUSDT --input ./data/raw --output ./data/features --from 2025-01-01 --to 2025-01-01`
3. Backtest
   - `cargo run -p backtester -- -s BTCUSDT --features ./data/features --from 2025-01-01 --to 2025-01-01 --report ./reports/quick.csv`
