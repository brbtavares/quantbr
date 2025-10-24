# Justfile for the QuantBR Rust workspace
# Docs: https://github.com/casey/just

# Use powershell on Windows
set windows-shell := ['pwsh', '-NoProfile', '-Command']

# Default recipe
_default:
    @just --list

# Helpers
set dotenv-load := false

# Variables
# Keep it simple: call cargo directly; override by invoking just with CARGO=cargo-nextest if desired
CARGO := 'cargo'
RUSTFLAGS := ''

# Workspace-wide tasks

# Build debug
build *ARGS:
    {{CARGO}} build --workspace --all-features {{ARGS}}

# Build release
release *ARGS:
    {{CARGO}} build --workspace --all-features --release {{ARGS}}

# Check (typecheck without building artifacts)
check *ARGS:
    {{CARGO}} check --workspace --all-features {{ARGS}}

# Test workspace
test *ARGS:
    {{CARGO}} test --workspace --all-features {{ARGS}}

# Format
fmt:
    {{CARGO}} fmt --all

# Format check (CI)
fmt-check:
    {{CARGO}} fmt --all -- --check

# Lint with clippy
clippy *ARGS:
    {{CARGO}} clippy --workspace --all-features -- -D warnings {{ARGS}}

# Lint alias
lint:
    just fmt-check
    just clippy

# Clean target
clean:
    {{CARGO}} clean

# Docs
doc *ARGS:
    {{CARGO}} doc --workspace --no-deps {{ARGS}}

# Run binaries (pass extra args after --)

# Backtester
run-backtester *ARGS:
    {{CARGO}} run -p backtester -- {{ARGS}}

# Collector
run-collector *ARGS:
    {{CARGO}} run -p collector -- {{ARGS}}

# Feature builder
run-feature-builder *ARGS:
    {{CARGO}} run -p feature_builder -- {{ARGS}}

# Fixtures
run-fixtures *ARGS:
    {{CARGO}} run -p fixtures -- {{ARGS}}

# Continuous integration aggregate
ci:
    just check
    just fmt-check
    just clippy
    just test
