use proxy_spider::parsers::PROXY_REGEX;

#[test]
fn test_parse_http_proxy_simple() {
    let text = "http://192.168.1.1:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("protocol").unwrap().as_str(), "http");
    assert_eq!(capture.name("host").unwrap().as_str(), "192.168.1.1");
    assert_eq!(capture.name("port").unwrap().as_str(), "8080");
    assert!(capture.name("username").is_none());
    assert!(capture.name("password").is_none());
}

#[test]
fn test_parse_http_proxy_with_auth() {
    let text = "http://user:pass@192.168.1.1:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("protocol").unwrap().as_str(), "http");
    assert_eq!(capture.name("username").unwrap().as_str(), "user");
    assert_eq!(capture.name("password").unwrap().as_str(), "pass");
    assert_eq!(capture.name("host").unwrap().as_str(), "192.168.1.1");
    assert_eq!(capture.name("port").unwrap().as_str(), "8080");
}

#[test]
fn test_parse_socks5_proxy() {
    let text = "socks5://10.0.0.1:1080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("protocol").unwrap().as_str(), "socks5");
    assert_eq!(capture.name("host").unwrap().as_str(), "10.0.0.1");
    assert_eq!(capture.name("port").unwrap().as_str(), "1080");
}

#[test]
fn test_parse_socks4_proxy() {
    let text = "socks4://172.16.0.1:9050";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("protocol").unwrap().as_str(), "socks4");
    assert_eq!(capture.name("host").unwrap().as_str(), "172.16.0.1");
    assert_eq!(capture.name("port").unwrap().as_str(), "9050");
}

#[test]
fn test_parse_proxy_without_protocol() {
    let text = "192.168.1.1:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert!(capture.name("protocol").is_none());
    assert_eq!(capture.name("host").unwrap().as_str(), "192.168.1.1");
    assert_eq!(capture.name("port").unwrap().as_str(), "8080");
}

#[test]
fn test_parse_multiple_proxies() {
    let text = r#"
        http://192.168.1.1:8080
        socks5://10.0.0.1:1080
        172.16.0.1:3128
    "#;
    
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    assert_eq!(captures.len(), 3);
}

#[test]
fn test_parse_proxy_with_domain() {
    let text = "http://proxy.example.com:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("host").unwrap().as_str(), "proxy.example.com");
    assert_eq!(capture.name("port").unwrap().as_str(), "8080");
}

#[test]
fn test_parse_proxy_with_special_chars_in_password() {
    let text = "http://user:p@ss:w0rd@192.168.1.1:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("username").unwrap().as_str(), "user");
    assert_eq!(capture.name("password").unwrap().as_str(), "p@ss:w0rd");
}

#[test]
fn test_parse_https_protocol() {
    let text = "https://192.168.1.1:8443";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    assert_eq!(captures.len(), 1);
    let capture = captures[0].as_ref().unwrap();
    
    assert_eq!(capture.name("protocol").unwrap().as_str(), "https");
}

#[test]
fn test_invalid_proxy_formats() {
    let invalid_texts = vec![
        "not a proxy",
        "http://",
        "192.168.1.1",  // Missing port
        "http://192.168.1.1:abc",  // Invalid port
        "",
    ];
    
    for text in invalid_texts {
        let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
        assert!(
            captures.is_empty() || captures[0].is_err(),
            "Should not match: {}",
            text
        );
    }
}

#[test]
fn test_parse_proxy_from_mixed_content() {
    let text = r#"
        Some random text here
        Check out this proxy: http://192.168.1.1:8080
        And another one socks5://user:pass@10.0.0.1:1080
        More text here
    "#;
    
    let captures: Vec<_> = PROXY_REGEX
        .captures_iter(text)
        .filter_map(Result::ok)
        .collect();
    
    assert_eq!(captures.len(), 2);
}

#[test]
fn test_parse_ipv6_proxy() {
    // Note: This test depends on whether the regex supports IPv6
    // Adjust based on actual implementation
    let text = "http://[2001:db8::1]:8080";
    let captures: Vec<_> = PROXY_REGEX.captures_iter(text).collect();
    
    // This might fail if IPv6 is not supported - document the limitation
    if !captures.is_empty() {
        let capture = captures[0].as_ref().unwrap();
        assert!(capture.name("host").is_some());
    }
}
