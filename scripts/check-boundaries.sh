#!/usr/bin/env bash
# Fail if the UI-free crates reference TUI dependencies.
set -euo pipefail
cd "$(dirname "$0")/.."
if grep -rnE 'ratatui|crossterm' crates; then
  echo "❌ lgtm-core/lgtm-mcp must not reference ratatui or crossterm" >&2
  exit 1
fi
echo "✅ Crate boundaries OK"
