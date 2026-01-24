use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use proxy_spider::config::{
    CheckingConfig, Config, JsonOutputConfig, OutputConfig, OutputFilters,
    ScrapingConfig, ServerConfig, TxtOutputConfig,
};
use proxy_spider::proxy::{Proxy, ProxyType};
use proxy_spider::server::{ProxyPool, start};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn create_test_config(addr: SocketAddr) -> Config {
    Config {
        debug: true,
        scraping: ScrapingConfig {
            max_proxies_per_source: 0,
            timeout: Duration::from_secs(1),
            connect_timeout: Duration::from_secs(1),
            proxy: None,
            user_agent: "test".to_string(),
            sources: proxy_spider::HashMap::default(),
        },
        checking: CheckingConfig {
            check_url: None,
            max_concurrent_checks: 1,
            timeout: Duration::from_secs(1),
            connect_timeout: Duration::from_secs(1),
            user_agent: "test".to_string(),
        },
        output: OutputConfig {
            path: PathBuf::from("./out-test-server"),
            sort_by_speed: false,
            txt: TxtOutputConfig { enabled: true, format: None },
            json: JsonOutputConfig {
                enabled: false,
                include_asn: false,
                include_geolocation: false,
            },
            rank: false,
            top: None,
            profile: None,
            filters: OutputFilters::default(),
        },
        server: ServerConfig {
            enabled: true,
            bind_addr: addr,
            tor_isolation: false,
            auth: None,
            rotation_method: "sequent".to_string(),
            rotate_after_requests: 1,
            rotate_on_error: false,
            remove_on_error: false,
            max_errors: None,
            max_redirs: None,
            max_retries: None,
            country_filter: None,
            sync: false,
            verbose: false,
            timeout: Duration::from_secs(1),
            output: None,
        },
    }
}

#[tokio::test]
async fn test_proxy_server_binding() {
    // Find a free port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let pool = ProxyPool::new();
    // Add a dummy proxy (it won't work for real traffic but enough to test server start)
    pool.update(vec![Proxy {
        protocol: ProxyType::Http,
        host: "127.0.0.1".to_string(),
        port: 80, // Dummy
        username: None,
        password: None,
        timeout: Some(Duration::from_millis(100)),
        exit_ip: None,
        anonymity: None,
        score: None,
    }]);

    let token = CancellationToken::new();
    let token_clone = token.clone();

    let config = Arc::new(create_test_config(addr));
    let server_handle = tokio::spawn(async move {
        start(config, pool, token_clone).await
    });

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Try to connect to the server
    let stream = tokio::net::TcpStream::connect(addr).await;
    assert!(stream.is_ok(), "Failed to connect to proxy server: {:?}", stream.err());

    // Shutdown
    token.cancel();
    let _result = server_handle.await;
}
