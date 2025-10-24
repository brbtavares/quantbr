# Contributing to quantbr

We welcome contributions! Please follow these guidelines.

- Use Conventional Commits (feat:, fix:, docs:, chore:, test:, refactor:, perf:, ci:)
- Format code with cargo fmt and lint with cargo clippy (no warnings)
- Add unit tests for new logic
- Keep public APIs documented with rustdoc

Example commit messages:
- feat(feature_builder): add ATR20 feature
- fix(backtester): correct fee application on exit

Before submitting a PR:
- cargo fmt --all
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace
