.PHONY: help build build-release build-all-features run run-release clean test bench check fmt fmt-check lint install dev docker-build docker-up docker-down

# Default target
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[0;33m
NC := \033[0m # No Color

help: ## Show this help message
	@echo "$(CYAN)proxy-spider Makefile$(NC)"
	@echo ""
	@echo "$(GREEN)Available targets:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2}'

# Build targets
build: ## Build the project in debug mode
	@echo "$(GREEN)Building proxy-spider (debug)...$(NC)"
	cargo build

build-release: ## Build the project in release mode with optimizations
	@echo "$(GREEN)Building proxy-spider (release)...$(NC)"
	cargo build --release

build-all-features: ## Build with all features enabled
	@echo "$(GREEN)Building proxy-spider (all features)...$(NC)"
	cargo build --all-features

build-tui: ## Build with TUI feature
	@echo "$(GREEN)Building proxy-spider (TUI)...$(NC)"
	cargo build --features tui

build-mimalloc: ## Build with mimalloc allocator
	@echo "$(GREEN)Building proxy-spider (mimalloc)...$(NC)"
	cargo build --release --features mimalloc

build-jemalloc: ## Build with jemalloc allocator
	@echo "$(GREEN)Building proxy-spider (jemalloc)...$(NC)"
	cargo build --release --features jemalloc

build-dhat: ## Build with dhat profiler
	@echo "$(GREEN)Building proxy-spider (dhat)...$(NC)"
	cargo build --profile dhat --features dhat

# Run targets
run: ## Run the project in debug mode
	@echo "$(GREEN)Running proxy-spider (debug)...$(NC)"
	cargo run

run-release: ## Run the project in release mode
	@echo "$(GREEN)Running proxy-spider (release)...$(NC)"
	cargo run --release

run-tui: ## Run with TUI feature
	@echo "$(GREEN)Running proxy-spider (TUI)...$(NC)"
	cargo run --features tui

# Testing targets
test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test

test-verbose: ## Run tests with verbose output
	@echo "$(GREEN)Running tests (verbose)...$(NC)"
	cargo test -- --nocapture

# Benchmarking
bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cargo bench

bench-proxy-parsing: ## Run proxy parsing benchmark
	@echo "$(GREEN)Running proxy parsing benchmark...$(NC)"
	cargo bench --bench proxy_parsing

bench-proxy-ops: ## Run proxy operations benchmark
	@echo "$(GREEN)Running proxy operations benchmark...$(NC)"
	cargo bench --bench proxy_operations

# Code quality
check: ## Run cargo check
	@echo "$(GREEN)Checking code...$(NC)"
	cargo check

check-all: ## Run cargo check with all features
	@echo "$(GREEN)Checking code (all features)...$(NC)"
	cargo check --all-features

clippy: ## Run clippy linter
	@echo "$(GREEN)Running clippy...$(NC)"
	cargo clippy --all-targets --all-features

clippy-fix: ## Run clippy with automatic fixes
	@echo "$(GREEN)Running clippy with fixes...$(NC)"
	cargo clippy --all-targets --all-features --fix --allow-dirty

fmt: ## Format code with rustfmt
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## Check code formatting without modifying files
	@echo "$(GREEN)Checking code formatting...$(NC)"
	cargo fmt -- --check

lint: clippy fmt-check ## Run all linters (clippy + fmt check)

# Cleaning
clean: ## Remove build artifacts
	@echo "$(YELLOW)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf out/

clean-all: clean ## Remove all generated files including output
	@echo "$(YELLOW)Cleaning all generated files...$(NC)"
	rm -rf target/ out/

# Installation
install: ## Install the binary to cargo bin directory
	@echo "$(GREEN)Installing proxy-spider...$(NC)"
	cargo install --path .

install-release: ## Install the release binary with optimizations
	@echo "$(GREEN)Installing proxy-spider (release)...$(NC)"
	cargo install --path . --profile release

# Development workflow
dev: fmt check test ## Run development checks (format, check, test)

ci: fmt-check clippy test ## Run CI checks (fmt-check, clippy, test)

# Docker targets
docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image...$(NC)"
	docker compose build

docker-build-linux: ## Build Docker image for Linux with user permissions
	@echo "$(GREEN)Building Docker image (Linux)...$(NC)"
	docker compose build --build-arg UID=$$(id -u) --build-arg GID=$$(id -g)

docker-up: ## Start Docker container
	@echo "$(GREEN)Starting Docker container...$(NC)"
	docker compose up --no-log-prefix --remove-orphans

docker-down: ## Stop Docker container
	@echo "$(YELLOW)Stopping Docker container...$(NC)"
	docker compose down

docker-logs: ## View Docker container logs
	@echo "$(GREEN)Viewing Docker logs...$(NC)"
	docker compose logs -f

# Profiling
profile-dhat: build-dhat ## Run with dhat heap profiler
	@echo "$(GREEN)Running with dhat profiler...$(NC)"
	./target/dhat/proxy-spider
	@echo "$(GREEN)Profile saved to dhat-heap.json$(NC)"

# Quick commands
quick-check: ## Quick validation (fmt + clippy)
	@$(MAKE) fmt
	@$(MAKE) clippy

all: clean build test ## Clean, build, and test everything

# Binary location helpers
binary-debug: ## Show debug binary location
	@echo "$(CYAN)Debug binary:$(NC) ./target/debug/proxy-spider"

binary-release: ## Show release binary location
	@echo "$(CYAN)Release binary:$(NC) ./target/release/proxy-spider"
