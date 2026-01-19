use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use proxy_spider::server::{ProxyPool, start};
use proxy_spider::proxy::{Proxy, ProxyType};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

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
    }]);

    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    let server_handle = tokio::spawn(async move {
        start(addr, pool, token_clone, false).await
    });

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect to the server
    let stream = tokio::net::TcpStream::connect(addr).await;
    assert!(stream.is_ok(), "Failed to connect to proxy server");

    // Shutdown
    token.cancel();
    let _ = server_handle.await;
}
