# Terminal Not Restored on UI Error

## Problem

An error from the UI event loop could return before raw mode, alternate screen, cursor, and mouse capture were restored, leaving the interactive terminal in a broken state.

## Resolution

Run terminal restoration on every exit after raw mode is enabled. Preserve the UI failure and surface a restoration failure as well.

## Regression Coverage

Unit tests exercise cleanup on success and failure, including simultaneous UI and cleanup errors.
