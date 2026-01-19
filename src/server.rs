use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

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
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            proxies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn update(&self, new_proxies: Vec<Proxy>) {
        // We accept all proxies now, will filter in handle_connection if needed
        // But for rotation, we generally want checked proxies.
        let mut proxies = self.proxies.write().unwrap();
        *proxies = new_proxies;
        info!("Updated proxy pool with {} proxies", proxies.len());
    }

    pub fn get_random(&self) -> Option<Proxy> {
         use rand::prelude::IndexedRandom;
        let proxies = self.proxies.read().unwrap();
        proxies.choose(&mut rand::rng()).map(|p| Proxy {
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

pub async fn start(
    bind_addr: SocketAddr,
    pool: ProxyPool,
    shutdown: tokio_util::sync::CancellationToken,
    tor_isolation: bool,
) -> crate::Result<()> {
    let listener = TcpListener::bind(bind_addr)
        .await
        .wrap_err_with(|| format!("failed to bind to {}", bind_addr))?;

    info!("Proxy server listening on {} (Tor isolation: {})", bind_addr, tor_isolation);

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
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, pool, tor_isolation).await {
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
    tor_isolation: bool,
) -> color_eyre::Result<()> {
    // 1. Peek at the request to determine target
    let mut buf = [0u8; 4096];
    let n = client_stream.peek(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request_str = std::str::from_utf8(&buf[..n]).unwrap_or("");
    
    // Simple parsing for CONNECT or GET
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
        // Assume GET/POST etc.
        // We need to extract Host header or absolute URL
        // Simplest: use Host header
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
        if host.is_empty() {
             // Fallback or fail? If we are a proxy, we expect absolute URI or Host header.
             return Ok(());
        }
        ("HTTP", host, port)
    };
    
    // Explicitly cast or type hint if needed, but parser should yield u16 above.
    // The previous error was because `80` literal defaults to i32.
    // Ensure all paths return u16.
    let target_port: u16 = target_port;

    let proxy = match pool.get_random() {
        Some(p) => p,
        None => return Ok(()),
    };

    let upstream_addr = format!("{}:{}", proxy.host, proxy.port);
    debug!("Connecting to upstream {} for target {}:{}", upstream_addr, target_host, target_port);

    let mut upstream_stream = TcpStream::connect(&upstream_addr)
        .await
        .wrap_err("failed to connect to upstream")?;

    match proxy.protocol {
        crate::proxy::ProxyType::Socks5 => {
            // SOCKS5 Handshake
            // 1. Auth negotiation
            let auth_methods = if tor_isolation {
                vec![0x02] // Username/Password
            } else {
                vec![0x00] // No auth
            };
            
            let mut handshake = vec![0x05, auth_methods.len() as u8];
            handshake.extend(auth_methods);
            upstream_stream.write_all(&handshake).await?;
            
            let mut response = [0u8; 2];
            upstream_stream.read_exact(&mut response).await?;
            
            if response[0] != 0x05 {
                 return Err(color_eyre::eyre::eyre!("Invalid SOCKS5 version"));
            }

            if response[1] == 0x02 && tor_isolation {
                // Perform User/Pass Auth
                // Generate random credentials
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
                if auth_res[1] != 0x00 {
                    return Err(color_eyre::eyre::eyre!("SOCKS5 Authentication failed"));
                }
            } else if response[1] == 0xFF {
                return Err(color_eyre::eyre::eyre!("No acceptable SOCKS5 auth methods"));
            }

            // 2. Connect Request
            // CMD=0x01 (CONNECT), ATYP=0x03 (Domain)
            let mut connect_req = vec![0x05, 0x01, 0x00, 0x03];
            connect_req.push(target_host.len() as u8);
            connect_req.extend_from_slice(target_host.as_bytes());
            connect_req.extend_from_slice(&target_port.to_be_bytes());
            
            upstream_stream.write_all(&connect_req).await?;
            
            let mut connect_res = [0u8; 4];
            upstream_stream.read_exact(&mut connect_res).await?;
            
            if connect_res[1] != 0x00 {
                 return Err(color_eyre::eyre::eyre!("SOCKS5 Connection failed: {}", connect_res[1]));
            }
            
            // Consume remaining address data
            let addr_type = connect_res[3];
            match addr_type {
                0x01 => { // IPv4
                    let mut buf = [0u8; 4+2];
                    upstream_stream.read_exact(&mut buf).await?;
                }
                0x03 => { // Domain
                     let mut len = [0u8; 1];
                     upstream_stream.read_exact(&mut len).await?;
                     let mut buf = vec![0u8; len[0] as usize + 2];
                     upstream_stream.read_exact(&mut buf).await?;
                }
                0x04 => { // IPv6
                    let mut buf = [0u8; 16+2];
                    upstream_stream.read_exact(&mut buf).await?;
                }
                _ => return Err(color_eyre::eyre::eyre!("Unknown address type")),
            }
        }
        _ => {
            // For HTTP proxies, we just assume they handle the request line we send next.
            // But if it is CONNECT, we must handle the response "200 OK" from us? 
            // OR we just pipe.
            // If we are "dumb piping" to HTTP proxy, we send the original request.
        }
    }

    // Handshake done. Now pipe.
    
    // Important: If method is CONNECT, and we successfully connected to Upstream (SOCKS),
    // we must send "200 Connection Established" to the Client *before* piping?
    // - If Upstream is SOCKS: YES. The SOCKS handshake is done, stream is ready. We tell Client OK.
    // - If Upstream is HTTP: We just forward the CONNECT request, and Upstream sends 200 OK.
    
    if method == "CONNECT" && matches!(proxy.protocol, crate::proxy::ProxyType::Socks5) {
        client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
        
        // We have consumed the "CONNECT ..." from client_stream? No, we PEEKED it.
        // We must DISCARD it from the stream before piping.
        // Reading it into buffer.
        // Read until \r\n\r\n
        // This is tricky without a buffered reader.
        // Hack: Read `n` bytes that we peeked? 
        // We peeked 4096, but n bytes were valid.
        // Assuming the header fits in 4096 and we read it all.
        // We need to read exactly the header length.
        // Let's assume `request_str` contains the full header if it ends with \r\n\r\n
        
        // The `n` bytes we peeked might not be the full header or might contain body.
        // Proper way: Read until end of headers.
        
        // MVP: Just read `n` bytes if request_str contains \r\n\r\n.
        if let Some(pos) = request_str.find("\r\n\r\n") {
             let header_len = pos + 4;
             let mut header_buf = vec![0u8; header_len];
             client_stream.read_exact(&mut header_buf).await?;
        } else {
             // Header larger than 4kb? Abort.
             return Ok(());
        }
    } else if method == "HTTP" && matches!(proxy.protocol, crate::proxy::ProxyType::Socks5) {
        // We need to forward the original GET request.
        // But we peeked it. It is still in client_stream.
        // So just piping is fine.
    }

    let (mut ri, mut wi) = client_stream.split();
    let (mut ro, mut wo) = upstream_stream.split();

    let client_to_server = async {
        tokio::io::copy(&mut ri, &mut wo).await
    };

    let server_to_client = async {
        tokio::io::copy(&mut ro, &mut wi).await
    };

    tokio::try_join!(client_to_server, server_to_client)?;

    Ok(())
}
