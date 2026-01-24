//! Metrics module for observability

use metrics::{describe_counter, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

/// Initialize metrics recorder
pub fn init(listen_addr: Option<SocketAddr>) -> color_eyre::Result<()> {
    let builder = PrometheusBuilder::new();

    if let Some(addr) = listen_addr {
        builder.with_http_listener(addr).install().map_err(|e| {
            color_eyre::eyre::eyre!(
                "failed to install Prometheus recorder: {}",
                e
            )
        })?;
    } else {
        builder.install().map_err(|e| {
            color_eyre::eyre::eyre!(
                "failed to install Prometheus recorder: {}",
                e
            )
        })?;
    }

    register_metrics();
    Ok(())
}

fn register_metrics() {
    describe_counter!(
        "proxies_scraped_total",
        "Total number of proxies scraped by protocol"
    );
    describe_counter!(
        "proxies_checked_total",
        "Total number of proxies checked"
    );
    describe_counter!(
        "proxies_working_total",
        "Total number of working proxies found"
    );
    describe_histogram!(
        "proxy_check_duration_seconds",
        "Duration of proxy checks"
    );
    describe_histogram!(
        "scrape_duration_seconds",
        "Duration of scraping tasks by protocol"
    );
    describe_counter!(
        "scrape_errors_total",
        "Total number of scraping errors by protocol"
    );
    describe_counter!(
        "proxies_saved_total",
        "Total number of proxies saved by format"
    );
}
