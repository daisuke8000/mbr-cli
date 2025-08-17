# Makefile for mbr-cli
# Provides consistent commands for local development and CI

.PHONY: help build test clean fmt lint check ci all

# Default target
help:
	@echo "Available targets:"
	@echo "  build     - Build the project"
	@echo "  test      - Run all tests"
	@echo "  fmt       - Format code"
	@echo "  lint      - Run clippy linter"
	@echo "  check     - Run all quality checks (fmt + lint + test)"
	@echo "  ci        - Full CI pipeline (check + build)"
	@echo "  clean     - Clean build artifacts"
	@echo "  all       - Run ci target"

# Build the project
build:
	cargo build

# Build release version
build-release:
	cargo build --release

# Run all tests
test:
	cargo test

# Format code
fmt:
	cargo fmt

# Check code formatting (used in CI)
fmt-check:
	cargo fmt --check

# Run clippy linter
lint:
	cargo clippy

# Run full clippy check (used in CI)
lint-full:
	cargo clippy --all-targets --all-features

# Quality checks for local development
check: fmt lint test

# Quality checks for CI (non-modifying)
check-ci: fmt-check lint-full test

# Full CI pipeline
ci: check-ci build

# Clean build artifacts
clean:
	cargo clean

# Default target for 'make' without arguments
all: ci