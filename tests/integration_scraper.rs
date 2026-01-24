use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use proxy_spider::config::{Config, ScrapingConfig, CheckingConfig, OutputConfig, ServerConfig};
use proxy_spider::proxy::ProxyType;
use proxy_spider::HashMap;
use mockito::Server;

#[tokio::test]
async fn test_scrape_all_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;

    let body = r#"
        http://1.1.1.1:8080
        socks5://user:pass@2.2.2.2:1080
        invalid_line
        3.3.3.3:3128
    "#;

    let mock = server.mock("GET", "/proxies")
        .with_status(200)
        .with_body(body)
        .create_async()
        .await;

    let url = format!("{}/proxies", server.url());

    let scraping = ScrapingConfig {
        max_proxies_per_source: 0,
        timeout: std::time::Duration::from_secs(5),
        connect_timeout: std::time::Duration::from_secs(2),
        proxy: None,
        user_agent: "TestAgent".to_string(),
        sources: {
            let mut s = HashMap::default();
            let source = Arc::new(proxy_spider::config::Source {
                url: url.clone(),
                basic_auth: None,
                headers: None,
            });
            // Enable all protocols so that regex matches are not filtered out
            s.insert(ProxyType::Http, vec![source.clone()]);
            s.insert(ProxyType::Socks4, vec![]);
            s.insert(ProxyType::Socks5, vec![]);
            s
        },
    };

    let checking = CheckingConfig {
        check_url: None,
        max_concurrent_checks: 1,
        timeout: std::time::Duration::from_secs(1),
        connect_timeout: std::time::Duration::from_secs(1),
        user_agent: "TestAgent".to_string(),
    };

    let config = Arc::new(Config {
        debug: true,
        scraping,
        checking,
        output: OutputConfig {
            path: std::path::PathBuf::from("./out-test"),
            sort_by_speed: false,
            txt: proxy_spider::config::TxtOutputConfig {
                enabled: false,
                format: None,
            },
            json: proxy_spider::config::JsonOutputConfig {
                enabled: false,
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
            bind_addr: "127.0.0.1:0".parse()?,
            tor_isolation: false,
            auth: None,
            rotation_method: "random".to_string(),
            rotate_after_requests: 1,
            rotate_on_error: false,
            remove_on_error: false,
            max_errors: None,
            max_redirs: None,
            max_retries: None,
            country_filter: None,
            sync: false,
            verbose: false,
            timeout: std::time::Duration::from_secs(1),
            output: None,
        },
    });

    let token = CancellationToken::new();
    let dns_resolver = Arc::new(proxy_spider::resolver::HickoryDnsResolver::new());
    let http_client = proxy_spider::http::create_client(&config, dns_resolver, None)?;

    #[cfg(feature = "tui")]
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<proxy_spider::event::Event>();
    
    let proxies = proxy_spider::scraper::scrape_all(
        config,
        http_client,
        token,
        #[cfg(feature = "tui")]
        tx,
    ).await?;

    mock.assert_async().await;

    assert_eq!(proxies.len(), 3);
    
    let protocols: Vec<_> = proxies.iter().map(|p| p.protocol).collect();
    assert!(protocols.contains(&ProxyType::Http));
    assert!(protocols.contains(&ProxyType::Socks5));

    Ok(())
}

#[tokio::test]
async fn test_scrape_deduplication() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;

    let body = r#"
        1.1.1.1:8080
        1.1.1.1:8080
        http://1.1.1.1:8080
    "#;

    let mock = server.mock("GET", "/dup")
        .with_status(200)
        .with_body(body)
        .create_async()
        .await;

    let url = format!("{}/dup", server.url());

    let scraping = ScrapingConfig {
        max_proxies_per_source: 0,
        timeout: std::time::Duration::from_secs(5),
        connect_timeout: std::time::Duration::from_secs(2),
        proxy: None,
        user_agent: "TestAgent".to_string(),
        sources: {
            let mut s = HashMap::default();
            s.insert(ProxyType::Http, vec![Arc::new(proxy_spider::config::Source {
                url,
                basic_auth: None,
                headers: None,
            })]);
            s
        },
    };

    // Minimal config for the rest
    let config = Arc::new(Config {
        debug: false,
        scraping,
        checking: CheckingConfig {
            check_url: None,
            max_concurrent_checks: 1,
            timeout: std::time::Duration::from_secs(1),
            connect_timeout: std::time::Duration::from_secs(1),
            user_agent: "Test".to_string(),
        },
        output: OutputConfig {
            path: std::path::PathBuf::from("./out-test"),
            sort_by_speed: false,
            txt: proxy_spider::config::TxtOutputConfig {
                enabled: false,
                format: None,
            },
            json: proxy_spider::config::JsonOutputConfig {
                enabled: false,
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
            bind_addr: "127.0.0.1:0".parse()?,
            tor_isolation: false,
            auth: None,
            rotation_method: "random".to_string(),
            rotate_after_requests: 1,
            rotate_on_error: false,
            remove_on_error: false,
            max_errors: None,
            max_redirs: None,
            max_retries: None,
            country_filter: None,
            sync: false,
            verbose: false,
            timeout: std::time::Duration::from_secs(1),
            output: None,
        },
    });

    let token = CancellationToken::new();
    let dns_resolver = Arc::new(proxy_spider::resolver::HickoryDnsResolver::new());
    let http_client = proxy_spider::http::create_client(&config, dns_resolver, None)?;

    #[cfg(feature = "tui")]
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<proxy_spider::event::Event>();

    let proxies = proxy_spider::scraper::scrape_all(
        config,
        http_client,
        token,
        #[cfg(feature = "tui")]
        tx,
    ).await?;

    mock.assert_async().await;

    // Should only have 1 proxy due to deduplication
    assert_eq!(proxies.len(), 1);
    assert_eq!(proxies[0].host, "1.1.1.1");
    assert_eq!(proxies[0].port, 8080);

    Ok(())
}
