# Metrics Documentation

`proxy-spider` uses the `metrics` crate for instrumenting core components and `metrics-exporter-prometheus` for exposing them.

## Available Metrics

### Scraper Metrics
- `proxies_scraped_total` (counter): Total number of proxies scraped, labeled by `protocol`.
- `scrape_duration_seconds` (histogram): Time taken to scrape a source, labeled by `protocol`.
- `scrape_errors_total` (counter): Number of failed scraping tasks, labeled by `protocol`.

### Checker Metrics
- `proxies_checked_total` (counter): Total number of proxies that entered the checker.
- `proxies_working_total` (counter): Total number of working proxies found, labeled by `protocol`.
- `proxy_check_duration_seconds` (histogram): Latency of successful proxy checks, labeled by `protocol`.

### Output Metrics
- `proxies_saved_total` (counter): Total number of proxies successfully written to files, labeled by `format` (e.g., `json`, `txt`).

## Enabling the Metrics Endpoint

By default, metrics are collected in memory but not exposed via HTTP. To enable the Prometheus export endpoint, you can either:

1.  **Modify `src/main.rs`**: Change the `metrics::init(None)` call to specify a socket address, e.g., `metrics::init(Some("127.0.0.1:9000".parse().unwrap()))`.
2.  **Environment Variable (Proposed)**: Future versions will support enabling this via an environment variable or config setting.

## Consuming Metrics

Once the endpoint is enabled (e.g., on port 9000), you can view metrics by visiting:
`http://127.0.0.1:9000/metrics`
