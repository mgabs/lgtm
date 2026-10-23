#!/usr/bin/env bash
set -eo pipefail

MIN_COVERAGE=${1:-90.0}

echo "=================================================="
echo " Running LGTM Test Suite & Fixed Module-Metric Gate"
echo " Fixed Module-Metric Threshold: ${MIN_COVERAGE}%"
echo "=================================================="

# 1. Run full cargo test suite
cargo test --workspace -- --nocapture

# 2. Run LGTM binary coverage audit gate
cargo run -- coverage --min "${MIN_COVERAGE}"

echo ""
echo "🎉 Tests and fixed module-metric gate passed; source coverage was not measured."
