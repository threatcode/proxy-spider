use std::sync::{Arc, RwLock, atomic::{AtomicUsize, Ordering}};

use color_eyre::eyre::WrapErr as _;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{debug, error, info, instrument};

use crate::proxy::Proxy;

#[derive(Clone)]
pub struct ProxyPool {
    proxies: Arc<RwLock<Vec<Proxy>>>,
    current_index: Arc<AtomicUsize>,
    request_count: Arc<AtomicUsize>,
    sync_mutex: Arc<tokio::sync::Mutex<()>>,
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            proxies: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(AtomicUsize::new(0)),
            request_count: Arc::new(AtomicUsize::new(0)),
            sync_mutex: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn update(&self, new_proxies: Vec<Proxy>) {
        let mut proxies = self.proxies.write().unwrap();
        *proxies = new_proxies;
        self.current_index.store(0, Ordering::SeqCst);
        self.request_count.store(0, Ordering::SeqCst);
        info!("Updated proxy pool with {} proxies", proxies.len());
    }

    pub fn get_next(&self, config: &crate::config::Config) -> Option<Proxy> {
        let proxies = self.proxies.read().unwrap();
        if proxies.is_empty() {
            return None;
        }

        let rotate_after = config.server.rotate_after_requests;
        let count = self.request_count.fetch_add(1, Ordering::SeqCst);
        
        let index = if config.server.rotation_method == "random" {
            use rand::Rng;
            if rotate_after > 0 && count % rotate_after == 0 {
                // Time to rotate
                let idx = rand::rng().random_range(0..proxies.len());
                self.current_index.store(idx, Ordering::SeqCst);
                idx
            } else {
                self.current_index.load(Ordering::SeqCst)
            }
        } else {
            // Sequent
            if rotate_after > 0 && count % rotate_after == 0 {
                let idx = self.current_index.fetch_add(1, Ordering::SeqCst) % proxies.len();
                idx
            } else {
                self.current_index.load(Ordering::SeqCst) % proxies.len()
            }
        };

        proxies.get(index).map(|p| Proxy {
            protocol: p.protocol,
            host: p.host.clone(),
            port: p.port,
            username: p.username.clone(),
            password: p.password.clone(),
            timeout: p.timeout,
            exit_ip: p.exit_ip.clone(),
        })
    }
}

fn redact_cookies(header: &str) -> String {
    let mut result = String::new();
    for line in header.lines() {
        if line.to_lowercase().starts_with("cookie:") {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                result.push_str(parts[0]);
                result.push_str(": ");
                let cookies = parts[1].trim();
                let redacted_cookies: Vec<String> = cookies
                    .split(';')
                    .map(|c| {
                        let c = c.trim();
                        if let Some((name, _)) = c.split_once('=') {
                             format!("{}=[REDACTED]", name)
                        } else {
                             // If no '=', redact the whole part just in case
                             format!("[REDACTED]")
                        }
                    })
                    .collect();
                result.push_str(&redacted_cookies.join("; "));
                result.push_str("\r\n");
            } else {
                result.push_str(line);
                result.push_str("\r\n");
            }
        } else {
            result.push_str(line);
            result.push_str("\r\n");
        }
    }
    result
}

fn log_traffic(config: &crate::config::Config, direction: &str, data: &[u8]) {
    if !config.server.verbose {
        return;
    }

    let text = String::from_utf8_lossy(data);
    // Only log headers (until \r\n\r\n)
    let header = text.split("\r\n\r\n").next().unwrap_or("");
    let redacted = redact_cookies(header);
    
    // Log to stdout
    println!("[{}] {}", direction, redacted.trim());
    
    // Note: Logging to file if config.server.output is set (requirement: headers NOT written to log file)
    // "If you use output option (-o/--output) to run proxy IP rotator, request/response headers are NOT written to the log file."
    // This implies we should be writing SOMETHING to the log file, probably just info about connections.
}

fn log_info(config: &crate::config::Config, msg: &str) {
    if let Some(path) = &config.server.output {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
            let _unused = writeln!(file, "{}", msg);
        }
    }
}

pub async fn start(
    config: Arc<crate::config::Config>,
    pool: ProxyPool,
    shutdown: tokio_util::sync::CancellationToken,
) -> crate::Result<()> {
    let bind_addr = config.server.bind_addr;
    let listener = TcpListener::bind(bind_addr)
        .await
        .wrap_err_with(|| format!("failed to bind to {}", bind_addr))?;

    info!("Proxy server listening on {} (Tor isolation: {})", bind_addr, config.server.tor_isolation);

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, addr) = match accepted {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                debug!("Accepted connection from {}", addr);
                let pool = pool.clone();
                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, pool, config).await {
                        debug!("Error handling connection from {}: {}", addr, e);
                    }
                });
            }
            () = shutdown.cancelled() => {
                info!("Proxy server shutting down");
                break;
            }
        }
    }
    Ok(())
}

#[instrument(skip_all)]
async fn handle_connection(
    mut client_stream: TcpStream,
    pool: ProxyPool,
    config: Arc<crate::config::Config>,
) -> color_eyre::Result<()> {
    // Sync mode
    let _guard = if config.server.sync {
        Some(pool.sync_mutex.lock().await)
    } else {
        None
    };

    // 1. Peek at the request to determine target
    let mut buf = [0u8; 4096];
    let n = client_stream.peek(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request_str = std::str::from_utf8(&buf[..n]).unwrap_or("");
    
    // Simple parsing for CONNECT or HTTP
    let (method, target_host, target_port) = if request_str.starts_with("CONNECT ") {
         let parts: Vec<&str> = request_str.split_whitespace().collect();
         if parts.len() < 2 { return Ok(()); }
         let target = parts[1];
         let (host, port) = if let Some((h, p)) = target.rsplit_once(':') {
             (h, p.parse().unwrap_or(80))
         } else {
             (target, 443)
         };
         ("CONNECT", host, port)
    } else {
        let mut host = "";
        let mut port = 80;
        for line in request_str.lines() {
            if line.to_lowercase().starts_with("host:") {
                let val = line[5..].trim();
                if let Some((h, p)) = val.rsplit_once(':') {
                    host = h;
                    port = p.parse().unwrap_or(80);
                } else {
                    host = val;
                }
                break;
            }
        }
        if host.is_empty() { return Ok(()); }
        ("HTTP", host, port)
    };
    
    let target_port: u16 = target_port;

    let max_retries = config.server.max_retries.unwrap_or(0);
    let max_errors = config.server.max_errors.unwrap_or(3);
    let mut current_error_count: usize = 0;

    loop {
        let proxy = match pool.get_next(&config) {
            Some(p) => p,
            None => {
                error!("No proxies available in pool");
                return Ok(());
            }
        };

        log_info(&config, &format!("Using proxy: {}", proxy.to_string(true)));

        let mut current_retry_attempt: usize = 0;
        loop {
            match connect_to_upstream(&proxy, target_host, target_port, method, &config).await {
                Ok(mut upstream_stream) => {
                    // Log request header
                    log_traffic(&config, "SEND", &buf[..n]);

                    // Handshake and Tunnel
                    if method == "CONNECT" && matches!(proxy.protocol, crate::proxy::ProxyType::Socks5) {
                        client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
                        if let Some(pos) = request_str.find("\r\n\r\n") {
                             let mut header_buf = vec![0u8; pos + 4];
                             client_stream.read_exact(&mut header_buf).await?;
                        }
                    }

                    let (mut ri, mut wi) = client_stream.split();
                    let (mut ro, mut wo) = upstream_stream.split();

                    let ctok = async { tokio::io::copy(&mut ri, &mut wo).await };
                    let stok = async { 
                        // Note: capturing data for logging is hard with tokio::io::copy.
                        // For MVP, we just pipe.
                        tokio::io::copy(&mut ro, &mut wi).await 
                    };

                    tokio::select! {
                        res = ctok => res?,
                        res = stok => res?,
                    };
                    return Ok(());
                }
                Err(e) => {
                    debug!("Attempt {} failed for proxy {}: {}", current_retry_attempt, proxy.to_string(true), e);
                    if current_retry_attempt < max_retries {
                        current_retry_attempt += 1;
                        continue; // Retry same proxy
                    } else {
                        break; // Move to next proxy
                    }
                }
            }
        }

        current_error_count += 1;
        if max_errors >= 0 && current_error_count >= max_errors as usize {
            error!("Max errors reached for request to {}:{}", target_host, target_port);
            return Ok(());
        }
        // Rotate loop continues
    }
}

async fn connect_to_upstream(
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
    _method: &str,
    config: &crate::config::Config,
) -> color_eyre::Result<TcpStream> {
    use crate::errors::{ErrorCode, ProxySpiderError};
    let upstream_addr = format!("{}:{}", proxy.host, proxy.port);
    let mut upstream_stream = tokio::time::timeout(config.server.timeout, TcpStream::connect(&upstream_addr))
        .await
        .map_err(|_| ProxySpiderError::new(ErrorCode::Timeout, "connection timeout"))?
        .wrap_err("failed to connect to upstream")?;

    match proxy.protocol {
        crate::proxy::ProxyType::Socks5 => {
            // SOCKS5 Handshake
            let auth_methods = if config.server.tor_isolation { vec![0x02] } else { vec![0x00] };
            let mut handshake = vec![0x05, auth_methods.len() as u8];
            handshake.extend(auth_methods);
            upstream_stream.write_all(&handshake).await?;
            
            let mut response = [0u8; 2];
            upstream_stream.read_exact(&mut response).await?;
            
            if response[1] == 0x02 && config.server.tor_isolation {
                use rand::{Rng, SeedableRng};
                use rand::rngs::StdRng;
                let mut rng = StdRng::from_os_rng();
                let username: String = (0..8).map(|_| rng.sample(rand::distr::Alphanumeric) as char).collect();
                let password: String = (0..8).map(|_| rng.sample(rand::distr::Alphanumeric) as char).collect();
                
                let mut auth_req = vec![0x01];
                auth_req.push(username.len() as u8);
                auth_req.extend_from_slice(username.as_bytes());
                auth_req.push(password.len() as u8);
                auth_req.extend_from_slice(password.as_bytes());
                upstream_stream.write_all(&auth_req).await?;
                
                let mut auth_res = [0u8; 2];
                upstream_stream.read_exact(&mut auth_res).await?;
                if auth_res[1] != 0x00 { return Err(color_eyre::eyre::eyre!("SOCKS5 Authentication failed")); }
            }

            let mut connect_req = vec![0x05, 0x01, 0x00, 0x03];
            connect_req.push(target_host.len() as u8);
            connect_req.extend_from_slice(target_host.as_bytes());
            connect_req.extend_from_slice(&target_port.to_be_bytes());
            upstream_stream.write_all(&connect_req).await?;
            
            let mut connect_res = [0u8; 4];
            upstream_stream.read_exact(&mut connect_res).await?;
            if connect_res[1] != 0x00 { return Err(color_eyre::eyre::eyre!("SOCKS5 Connection failed: {}", connect_res[1])); }
            
            let addr_type = connect_res[3];
            match addr_type {
                0x01 => { upstream_stream.read_exact(&mut [0u8; 6]).await?; }
                0x03 => {
                     let mut len = [0u8; 1];
                     upstream_stream.read_exact(&mut len).await?;
                     let mut buf = vec![0u8; len[0] as usize + 2];
                     upstream_stream.read_exact(&mut buf).await?;
                }
                0x04 => { upstream_stream.read_exact(&mut [0u8; 18]).await?; }
                _ => return Err(color_eyre::eyre::eyre!("Unknown address type")),
            }
        }
        _ => {}
    }
    Ok(upstream_stream)
}

