# API Documentation

This document provides an overview of the public APIs provided by the `proxy-spider` library.

## Core Modules

### `proxy`
Defined in `src/proxy.rs`. Contains the core data models and proxy checking logic.
- `Proxy`: The main struct representing a proxy server.
- `ProxyType`: Enum for supported protocols (HTTP, SOCKS4, SOCKS5).

### `scraper`
Defined in `src/scraper.rs`. Coordinates the scraping of proxies from various sources.
- `scrape_all`: Main entry point for parallel scraping.

### `checker`
Defined in `src/checker.rs`. Handles the concurrent checking of scraped proxies.
- `check_all`: Main entry point for parallel proxy validation.

### `config`
Defined in `src/config.rs`. Manages configuration loading and validation.
- `Config`: The processed configuration struct.
- `load_config`: Utilities for reading from file system.

### `server`
Defined in `src/server.rs`. Implements the optional proxy rotation server.

## Example Usage

See the `examples/` directory for detailed usage examples:
- `basic_usage.rs`: Loading default config and running the full pipeline.
- `custom_config.rs`: Programmatically building a complex configuration.
