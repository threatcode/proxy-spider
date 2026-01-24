use proxy_spider::raw_config::RawConfig;
use proxy_spider::config::Config;

fn create_raw_config() -> RawConfig {
    let toml_str = r#"
debug = false
[scraping]
max_proxies_per_source = 0
timeout = 5.0
connect_timeout = 2.0
user_agent = "Test"
proxy = ""
[scraping.http]
enabled = true
urls = ["http://example.com/proxies"]
[scraping.socks4]
enabled = false
urls = []
[scraping.socks5]
enabled = false
urls = []
[checking]
check_url = "http://api.ipify.org"
max_concurrent_checks = 10
timeout = 5.0
connect_timeout = 2.0
user_agent = "Test"
[output]
path = "./out-test"
sort_by_speed = false
[output.txt]
enabled = true
[output.json]
enabled = false
include_asn = false
include_geolocation = false
"#;
    toml::from_str(toml_str).expect("Failed to parse mock TOML")
}

#[tokio::test]
async fn test_config_validation_valid() {
    let raw_config = create_raw_config();
    let config = Config::from_raw_config(raw_config).await;
    match &config {
        Ok(_) => (),
        Err(e) => panic!("Config validation failed: {e:?}"),
    }
    assert!(config.is_ok());
}

#[tokio::test]
async fn test_config_validation_bad_values() {
    let mut raw_config = create_raw_config();
    // Connect timeout must be > 0.0 (per validation_positive_f64)
    raw_config.scraping.connect_timeout = -1.0;
    
    let res = Config::from_raw_config(raw_config).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_config_loading() {
    let config_content = r#"
debug = true
[scraping]
max_proxies_per_source = 0
timeout = 10.0
connect_timeout = 2.0
user_agent = "TestAgent"
proxy = ""
[scraping.http]
enabled = true
urls = ["http://example.com/proxies"]
[scraping.socks4]
enabled = false
urls = []
[scraping.socks5]
enabled = false
urls = []
[checking]
check_url = "http://api.ipify.org"
max_concurrent_checks = 10
timeout = 5.0
connect_timeout = 2.0
user_agent = "Test"
[output]
path = "./out-test"
sort_by_speed = false
[output.txt]
enabled = true
[output.json]
enabled = false
include_asn = false
include_geolocation = false
"#;
    
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_config_final_2.toml");
    std::fs::write(&config_path, config_content).unwrap();
    
    let raw = proxy_spider::raw_config::read_config(&config_path).await.unwrap();
    let config = Config::from_raw_config(raw).await.unwrap();
    
    assert!(config.debug);
    assert_eq!(config.scraping.timeout.as_secs(), 10);
    assert_eq!(config.scraping.user_agent, "TestAgent");
    
    std::fs::remove_file(config_path).unwrap();
}
