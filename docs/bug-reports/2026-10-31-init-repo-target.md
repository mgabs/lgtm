# `--init` Ignored the Selected Repository

## Problem

The early `--init` shortcut initialized the current directory before parsing arguments, ignoring an explicit `--repo` target.

## Resolution

Handle `--init` after CLI parsing and use the parsed repository path, preserving the current-directory default.

## Regression Coverage

BDD coverage exercises relative and absolute `--repo` values, the default target, and preserving an existing configuration.
