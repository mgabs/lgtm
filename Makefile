.PHONY: all build release test coverage mutate lint fmt check risk ci install install-hooks clean help

SHELL := /bin/bash
TARGET_BIN := target/release/lgtm
DEV_BIN := target/debug/lgtm
MIN_COVERAGE := 90.0
MIN_MUTATION := 90.0

all: test coverage mutate release

## Build development binary
build:
	cargo build

## Build optimized release binary with LTO & strip
release:
	cargo build --release
	@echo "✨ Release binary built at $(TARGET_BIN)"

## Run all unit and integration tests
test:
	cargo test --workspace -- --nocapture

## Run test coverage audit and enforce >= 90% threshold
coverage:
	./scripts/coverage.sh $(MIN_COVERAGE)

## Run diff-aware delta coverage audit strictly on changed lines
delta-coverage:
	cargo run -- coverage --delta --min $(MIN_COVERAGE)

## Run automated mutation testing and enforce >= 90% score
mutate:
	./scripts/mutate.sh $(MIN_MUTATION)

## Run inverted test verification to detect fraudulent or tautological tests
verify-tests:
	cargo run -- verify-tests

## Run clippy linter and fail on warnings
lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	./scripts/check-boundaries.sh

## Format Rust codebase
fmt:
	cargo fmt

## Check formatting without modifying files
check-fmt:
	cargo fmt --all -- --check

## Combined quality gate (fmt check + clippy + test + coverage + mutate)
check: check-fmt lint test coverage mutate

## Run headless security & test fraud audit on working tree diff
risk:
	cargo run -- risk

## Run full CI pipeline (tests, coverage, mutation, SARIF report)
ci: check
	-cargo run -- ci --format sarif --output lgtm-results.sarif
	@echo "✨ Generated SARIF report at lgtm-results.sarif"

## Install git pre-commit hooks
install-hooks:
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
	@echo "✨ Git hooks installed (.githooks/pre-commit)"

## Install release binary to ~/.cargo/bin
install: release
	cargo install --path . --force

## Clean build artifacts and temporary files
clean:
	cargo clean
	rm -f lgtm-results.sarif .lgtm/review-*.md .lgtm/pr-*-review.md

## Update local Homebrew tap formula
tap:
	./scripts/update-tap.sh ../homebrew-tap

## Show available Makefile targets
help:
	@echo "Available make targets:"
	@echo "  make build       - Build debug binary"
	@echo "  make release     - Build optimized release binary"
	@echo "  make test        - Run full test suite"
	@echo "  make coverage    - Run coverage enforcer (>= $(MIN_COVERAGE)%)"
	@echo "  make mutate      - Run mutation tester (>= $(MIN_MUTATION)%)"
	@echo "  make lint        - Run cargo clippy"
	@echo "  make fmt         - Format codebase with rustfmt"
	@echo "  make check       - Full pre-commit check (fmt + lint + test + cov + mut)"
	@echo "  make risk        - Run security & test guard audit"
	@echo "  make ci          - Run CI pipeline with SARIF report"
	@echo "  make tap         - Update local Homebrew tap formula"
	@echo "  make install-hooks - Install git pre-commit hooks"
	@echo "  make install     - Install binary to cargo path"
	@echo "  make clean       - Remove target/ and artifacts"

