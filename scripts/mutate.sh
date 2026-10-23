#!/usr/bin/env bash
set -eo pipefail

MIN_SCORE=${1:-90.0}

echo "========================================================"
echo " Running LGTM Test Suite & Fixed Mutation Catalog"
echo " Minimum Catalog Score: ${MIN_SCORE}% (zero survivors required)"
echo "========================================================"

# 1. Run full unit & mutation test suite
cargo test --workspace -- --nocapture

# 2. Run mutation audit gate
cargo run -- mutate --min "${MIN_SCORE}"

echo ""
echo "🎉 Fixed mutation catalog passed (0 survivors); source mutations were not generated."
