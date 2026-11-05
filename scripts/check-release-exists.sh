#!/usr/bin/env bash
set -euo pipefail

# Check if a release tag exists via gh CLI or git tags.
# Outputs exists=true or exists=false to stdout.

TAG="${1:-}"
if [ -z "$TAG" ]; then
  VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f2)
  TAG="v${VERSION}"
fi

if command -v gh >/dev/null 2>&1 && gh release view "$TAG" >/dev/null 2>&1; then
  echo "exists=true"
  exit 0
elif git rev-parse -q --verify "refs/tags/$TAG" >/dev/null 2>&1; then
  echo "exists=true"
  exit 0
else
  echo "exists=false"
  exit 0
fi
