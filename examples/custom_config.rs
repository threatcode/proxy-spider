//! Custom configuration example for `proxy-spider`.
//!
//! This example demonstrates how to programmatically create a configuration,
//! customize its settings (like timeouts and sources), and run the task.

use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use url::Url;

use proxy_spider::config::{Config, ScrapingConfig, CheckingConfig, OutputConfig, ServerConfig};
use proxy_spider::proxy::ProxyType;
use proxy_spider::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt::init();

    // 2. Define custom scraping configuration
    let scraping = ScrapingConfig {
        max_proxies_per_source: 100,
        timeout: Duration::from_secs(30),
        connect_timeout: Duration::from_secs(5),
        proxy: None,
        user_agent: "Mozilla/5.0 (Custom Agent)".to_string(),
        sources: {
            let mut s = HashMap::default();
            s.insert(ProxyType::Http, vec![Arc::new(proxy_spider::config::Source {
                url: "https://api.proxyscrape.com/v2/?request=getproxies&protocol=http&timeout=10000&country=all&ssl=all&anonymity=all".to_string(),
                basic_auth: None,
                headers: None,
            })]);
            s
        },
    };

    // 3. Define custom checking configuration
    let checking = CheckingConfig {
        check_url: Some(Url::parse("https://api.ipify.org")?),
        max_concurrent_checks: 512,
        timeout: Duration::from_secs(10),
        connect_timeout: Duration::from_secs(5),
        user_agent: "Mozilla/5.0 (Custom Agent)".to_string(),
    };

    // 4. Combine into complete Config
    let config = Arc::new(Config {
        debug: false,
        scraping,
        checking,
        output: OutputConfig {
            path: std::path::PathBuf::from("./out-custom"),
            sort_by_speed: true,
            txt: proxy_spider::config::TxtOutputConfig { enabled: true, format: None },
            json: proxy_spider::config::JsonOutputConfig {
                enabled: true,
                include_asn: false,
                include_geolocation: false,
            },
            rank: false,
            top: None,
            profile: None,
            filters: proxy_spider::config::OutputFilters::default(),
        },
        server: ServerConfig {
            enabled: false,
            bind_addr: "127.0.0.1:8080".parse()?,
            tor_isolation: false,
            auth: None,
            rotation_method: "random".to_string(),
            rotate_after_requests: 1,
            rotate_on_error: false,
            remove_on_error: false,
            max_errors: Some(3),
            max_redirs: None,
            max_retries: None,
            country_filter: None,
            sync: false,
            verbose: false,
            timeout: Duration::from_secs(30),
            output: None,
        },
    });

    // 5. Create a cancellation token
    let token = CancellationToken::new();

    // 6. Run the task
    println!("Running with custom configuration...");
    proxy_spider::main_task(
        config,
        token,
        #[cfg(feature = "tui")]
        tokio::sync::mpsc::unbounded_channel().0,
    ).await?;

    println!("Custom run completed.");
    Ok(())
}
