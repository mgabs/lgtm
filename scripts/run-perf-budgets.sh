#!/usr/bin/env bash
set -eo pipefail

echo "Running performance budget benchmarks in release mode..."
cargo test -p lgtm --release --features perf-budgets test_performance_budgets_nfr -- --nocapture
