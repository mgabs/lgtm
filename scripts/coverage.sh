#!/usr/bin/env bash
set -eo pipefail

MIN_COVERAGE=${1:-90.0}

echo "=================================================="
echo " Running LGTM Test Suite & Coverage Enforcer"
echo " Enforcing Minimum Coverage Threshold: ${MIN_COVERAGE}%"
echo "=================================================="

# 1. Run full cargo test suite
cargo test --workspace -- --nocapture

# 2. Run LGTM binary coverage audit gate
cargo run -- coverage --min "${MIN_COVERAGE}"

echo ""
echo "🎉 All tests and code coverage requirements successfully met!"
