use std::str::FromStr;

use proxy_spider::proxy::{Proxy, ProxyType};

#[test]
fn test_proxy_type_from_str_http() {
    assert_eq!(ProxyType::from_str("http").unwrap(), ProxyType::Http);
    assert_eq!(ProxyType::from_str("HTTP").unwrap(), ProxyType::Http);
    assert_eq!(ProxyType::from_str("https").unwrap(), ProxyType::Http);
    assert_eq!(ProxyType::from_str("HTTPS").unwrap(), ProxyType::Http);
}

#[test]
fn test_proxy_type_from_str_socks4() {
    assert_eq!(ProxyType::from_str("socks4").unwrap(), ProxyType::Socks4);
    assert_eq!(ProxyType::from_str("SOCKS4").unwrap(), ProxyType::Socks4);
}

#[test]
fn test_proxy_type_from_str_socks5() {
    assert_eq!(ProxyType::from_str("socks5").unwrap(), ProxyType::Socks5);
    assert_eq!(ProxyType::from_str("SOCKS5").unwrap(), ProxyType::Socks5);
}

#[test]
fn test_proxy_type_from_str_invalid() {
    assert!(ProxyType::from_str("invalid").is_err());
    assert!(ProxyType::from_str("").is_err());
    assert!(ProxyType::from_str("socks6").is_err());
}

#[test]
fn test_proxy_type_as_str() {
    assert_eq!(ProxyType::Http.as_str(), "http");
    assert_eq!(ProxyType::Socks4.as_str(), "socks4");
    assert_eq!(ProxyType::Socks5.as_str(), "socks5");
}

#[test]
fn test_proxy_to_string_without_protocol() {
    let proxy = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_eq!(proxy.to_string(false), "192.168.1.1:8080");
}

#[test]
fn test_proxy_to_string_with_protocol() {
    let proxy = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_eq!(proxy.to_string(true), "http://192.168.1.1:8080");
}

#[test]
fn test_proxy_to_string_with_auth() {
    let proxy = Proxy {
        protocol: ProxyType::Socks5,
        host: "10.0.0.1".to_string(),
        port: 1080,
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_eq!(proxy.to_string(false), "user:pass@10.0.0.1:1080");
    assert_eq!(proxy.to_string(true), "socks5://user:pass@10.0.0.1:1080");
}

#[test]
fn test_proxy_is_checked() {
    let mut proxy = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert!(!proxy.is_checked());

    proxy.timeout = Some(std::time::Duration::from_secs(1));
    assert!(proxy.is_checked());
}

#[test]
fn test_proxy_equality() {
    let proxy1 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    let proxy2 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: Some(std::time::Duration::from_secs(1)),
        exit_ip: Some("1.2.3.4".to_string()),
        anonymity: None,
        score: None,
    };

    // Proxies are equal if protocol, host, port, username, and password match
    // timeout and exit_ip are not considered
    assert_eq!(proxy1, proxy2);
}

#[test]
fn test_proxy_inequality_different_host() {
    let proxy1 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    let proxy2 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.2".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_ne!(proxy1, proxy2);
}

#[test]
fn test_proxy_inequality_different_port() {
    let proxy1 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    let proxy2 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8081,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_ne!(proxy1, proxy2);
}

#[test]
fn test_proxy_inequality_different_protocol() {
    let proxy1 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    let proxy2 = Proxy {
        protocol: ProxyType::Socks5,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    assert_ne!(proxy1, proxy2);
}

#[test]
fn test_proxy_hash_consistency() {
    use std::collections::HashSet;

    let proxy1 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
        anonymity: None,
        score: None,
    };

    let proxy2 = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: None,
        password: None,
        timeout: Some(std::time::Duration::from_secs(1)),
        exit_ip: Some("1.2.3.4".to_string()),
        anonymity: None,
        score: None,
    };

    let mut set = HashSet::new();
    set.insert(proxy1);

    // proxy2 should be considered a duplicate since it has the same hash
    assert!(!set.insert(proxy2));
    assert_eq!(set.len(), 1);
}

#[test]
fn test_proxy_deduplication() {
    use std::collections::HashSet;

    let proxies = vec![
        Proxy {
            protocol: ProxyType::Http,
            host: "192.168.1.1".to_string(),
            port: 8080,
            username: None,
            password: None,
            timeout: None,
            exit_ip: None,
            anonymity: None,
            score: None,
        },
        Proxy {
            protocol: ProxyType::Http,
            host: "192.168.1.1".to_string(),
            port: 8080,
            username: None,
            password: None,
            timeout: Some(std::time::Duration::from_secs(1)),
            exit_ip: None,
            anonymity: None,
            score: None,
        },
        Proxy {
            protocol: ProxyType::Http,
            host: "192.168.1.2".to_string(),
            port: 8080,
            username: None,
            password: None,
            timeout: None,
            exit_ip: None,
            anonymity: None,
            score: None,
        },
    ];

    let unique: HashSet<_> = proxies.into_iter().collect();
    assert_eq!(unique.len(), 2); // First two are duplicates
}
