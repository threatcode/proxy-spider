# Makefile Quick Reference

## 🚀 Most Common Commands

```bash
# Build and run
make build              # Build debug version
make run                # Run debug version
make build-release      # Build optimized release
make run-release        # Run optimized release

# Development workflow
make dev                # Format + check + test
make quick-check        # Format + clippy (fast validation)

# Code quality
make fmt                # Format code
make clippy             # Run linter
make test               # Run tests

# Clean up
make clean              # Remove build artifacts
```

## 📦 Build Variants

```bash
make build-tui          # With terminal UI
make build-mimalloc     # With mimalloc allocator (faster)
make build-jemalloc     # With jemalloc allocator
make build-all-features # All features enabled
```

## 🐳 Docker Commands

```bash
make docker-build       # Build Docker image
make docker-up          # Start container
make docker-down        # Stop container
make docker-logs        # View logs
```

## 🧪 Testing & Benchmarking

```bash
make test               # Run all tests
make test-verbose       # Tests with output
make bench              # Run all benchmarks
make bench-proxy-parsing # Specific benchmark
```

## 📊 Profiling

```bash
make profile-dhat       # Heap profiling with dhat
```

## 🔍 Full Command List

Run `make help` to see all 35+ available targets with descriptions.

## 💡 Tips

- **First time setup**: `make build && make test`
- **Before committing**: `make ci` (runs fmt-check, clippy, test)
- **Quick iteration**: `make quick-check` (faster than full CI)
- **Production build**: `make build-release` or `make install-release`
