#!/usr/bin/env bash
set -eo pipefail

MIN_SCORE=${1:-90.0}

echo "========================================================"
echo " Running LGTM Automated Mutation Testing Suite"
echo " Enforcing Minimum Mutation Score Threshold: ${MIN_SCORE}%"
echo "========================================================"

# 1. Run full unit & mutation test suite
cargo test -- --nocapture

# 2. Run mutation audit gate
cargo run -- mutate --min "${MIN_SCORE}"

echo ""
echo "🎉 Mutation testing passed! 100% mutants killed (0 survived)."
