//! Proxy output module.
//!
//! This module provides functionality for saving checked proxies in various formats
//! (plain text and JSON). It also handles sorting proxies by speed or naturally
//! and enriching JSON output with ASN and geolocation data.

use std::{
    cmp::Ordering,
    io,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::WrapErr as _;

use crate::{
    HashMap, HashSet,
    config::Config,
    ipdb,
    proxy::{Proxy, ProxyType},
    utils::is_docker,
};

fn compare_timeout(a: &Proxy, b: &Proxy) -> Ordering {
    a.timeout.unwrap_or(Duration::MAX).cmp(&b.timeout.unwrap_or(Duration::MAX))
}

fn compare_natural(a: &Proxy, b: &Proxy) -> Ordering {
    a.protocol
        .cmp(&b.protocol)
        .then_with(move || {
            match (a.host.parse::<Ipv4Addr>(), b.host.parse::<Ipv4Addr>()) {
                (Ok(ai), Ok(bi)) => ai.octets().cmp(&bi.octets()),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => a.host.cmp(&b.host),
            }
        })
        .then_with(move || a.port.cmp(&b.port))
}

#[derive(serde::Serialize)]
struct ProxyJson<'a> {
    protocol: ProxyType,
    username: Option<&'a str>,
    password: Option<&'a str>,
    host: &'a str,
    port: u16,
    timeout: Option<f64>,
    exit_ip: Option<&'a str>,
    hostname: Option<String>,
    anonymity: Option<crate::proxy::AnonymityLevel>,
    score: Option<u8>,
    asn: Option<maxminddb::geoip2::Asn<'a>>,
    geolocation: Option<maxminddb::geoip2::City<'a>>,
}

fn group_proxies<'a>(
    config: &Config,
    proxies: &'a [Proxy],
) -> HashMap<ProxyType, Vec<&'a Proxy>> {
    let mut groups: HashMap<_, _> =
        config.enabled_protocols().copied().map(|p| (p, Vec::new())).collect();
    for proxy in proxies {
        if let Some(proxies) = groups.get_mut(&proxy.protocol) {
            proxies.push(proxy);
        }
    }
    groups
}

/// Saves the collected proxies to the configured output files.
#[expect(clippy::too_many_lines)]
pub async fn save_proxies(
    config: Arc<Config>,
    mut proxies: Vec<Proxy>,
    dns_resolver: Arc<crate::resolver::HickoryDnsResolver>,
) -> crate::Result<()> {
    if config.output.sort_by_speed {
        proxies.sort_unstable_by(compare_timeout);
    } else {
        proxies.sort_unstable_by(compare_natural);
    }

    let needs_asn = config.output.json.include_asn
        || config.output.txt.format.as_ref().map_or(false, |f| f.contains("{{org}}"));
    let needs_geo = config.output.json.include_geolocation
        || config.output.txt.format.as_ref().map_or(false, |f| {
            f.contains("{{country}}")
                || f.contains("{{city}}")
                || f.contains("{{region}}")
                || f.contains("{{timezone}}")
                || f.contains("{{loc}}")
        })
        || config.output.filters.only_cc.is_some();
    let needs_hostname = config.output.json.enabled
        || config.output.txt.format.as_ref().map_or(false, |f| f.contains("{{hostname}}"));

    let (maybe_asn_db, maybe_geo_db): (
        Option<maxminddb::Reader<maxminddb::Mmap>>,
        Option<maxminddb::Reader<maxminddb::Mmap>>,
    ) = tokio::try_join!(
        async {
            if needs_asn {
                ipdb::DbType::Asn.open_mmap().await.map(Some)
            } else {
                Ok(None)
            }
        },
        async {
            if needs_geo {
                ipdb::DbType::Geo.open_mmap().await.map(Some)
            } else {
                Ok(None)
            }
        }
    )?;

    // Perform reverse DNS lookups if needed
    let mut hostnames = HashMap::default();
    if needs_hostname {
        let mut join_set = tokio::task::JoinSet::new();
        let mut seen_ips = HashSet::default();

        for proxy in &proxies {
            if let Some(exit_ip) = &proxy.exit_ip {
                if let Ok(ip_addr) = exit_ip.parse::<IpAddr>() {
                    if seen_ips.insert(ip_addr) {
                        let resolver = Arc::clone(&dns_resolver);
                        join_set.spawn(async move {
                            (ip_addr, resolver.reverse_lookup(ip_addr).await.ok().flatten())
                        });
                    }
                }
            }
        }

        while let Some(res) = join_set.join_next().await {
            if let Ok((ip, hostname)) = res {
                if let Some(name) = hostname {
                    hostnames.insert(ip.to_string(), name);
                }
            }
        }
    }

    // Apply filters
    filter_proxies(
        &config,
        &mut proxies,
        maybe_asn_db.as_ref(),
        maybe_geo_db.as_ref(),
    )?;

    // Ranking and Sorting
    if config.output.rank || config.output.profile.is_some() {
        proxies.sort_by(|a, b| {
            b.score
                .unwrap_or(0)
                .cmp(&a.score.unwrap_or(0))
                .then_with(|| {
                    // Fallback to latency for equal scores
                    a.timeout
                        .unwrap_or(Duration::MAX)
                        .cmp(&b.timeout.unwrap_or(Duration::MAX))
                })
        });
    } else if config.output.sort_by_speed {
        proxies.sort_by(|a, b| {
            a.timeout
                .unwrap_or(Duration::MAX)
                .cmp(&b.timeout.unwrap_or(Duration::MAX))
        });
    }

    // Top N truncation
    if let Some(top) = config.output.top {
        proxies.truncate(top);
    }

    if config.output.json.enabled {
        let mut proxy_dicts = Vec::with_capacity(proxies.len());
        for proxy in &proxies {
            let (asn, geolocation) =
                lookup_metadata(proxy, maybe_asn_db.as_ref(), maybe_geo_db.as_ref())?;
            let hostname = proxy.exit_ip.as_ref().and_then(|ip| hostnames.get(ip).cloned());
            
            proxy_dicts.push(ProxyJson {
                protocol: proxy.protocol,
                username: proxy.username.as_deref(),
                password: proxy.password.as_deref(),
                host: &proxy.host,
                port: proxy.port,
                timeout: proxy
                    .timeout
                    .map(|d| (d.as_secs_f64() * 100.0).round() / 100.0_f64),
                exit_ip: proxy.exit_ip.as_deref(),
                hostname,
                anonymity: proxy.anonymity,
                score: proxy.score,
                asn,
                geolocation,
            });
        }

        for (path, pretty) in [
            (config.output.path.join("proxies.json"), false),
            (config.output.path.join("proxies_pretty.json"), true),
        ] {
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e).wrap_err_with(|| {
                    format!("failed to remove file {}", path.display())
                }),
            }?;
            let json_data = if pretty {
                serde_json::to_vec_pretty(&proxy_dicts)
                    .wrap_err("failed to serialize proxies to pretty json")?
            } else {
                serde_json::to_vec(&proxy_dicts)
                    .wrap_err("failed to serialize proxies to json")?
            };
            tokio::fs::write(&path, json_data).await.wrap_err_with(
                move || {
                    format!("failed to write proxies to {}", path.display())
                },
            )?;
        }
        metrics::counter!("proxies_saved_total", "format" => "json")
            .increment(proxies.len() as u64);
    }

    if config.output.txt.enabled {
        let grouped_proxies = group_proxies(&config, &proxies);
        let directory_path = config.output.path.join("proxies");
        match tokio::fs::remove_dir_all(&directory_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).wrap_err_with(|| {
                format!(
                    "failed to remove directory {}",
                    directory_path.display()
                )
            }),
        }?;
        tokio::fs::create_dir_all(&directory_path).await.wrap_err_with(
            || {
                format!(
                    "failed to create directory: {}",
                    directory_path.display()
                )
            },
        )?;

        let format = config.output.txt.format.as_deref();

        write_proxy_list(
            &directory_path.join("all.txt"),
            proxies.iter(),
            true,
            format,
            maybe_asn_db.as_ref(),
            maybe_geo_db.as_ref(),
            &hostnames,
        )
        .await
        .wrap_err_with(|| {
            format!(
                "failed to write proxies to {}",
                directory_path.join("all.txt").display()
            )
        })?;

        for (proto, proxies) in grouped_proxies {
            let mut file_path = directory_path.join(proto.as_str());
            file_path.set_extension("txt");
            write_proxy_list(
                &file_path,
                proxies,
                false,
                format,
                maybe_asn_db.as_ref(),
                maybe_geo_db.as_ref(),
                &hostnames,
            )
            .await
            .wrap_err_with(move || {
                format!("failed to write proxies to {}", file_path.display())
            })?;
        }
        metrics::counter!("proxies_saved_total", "format" => "txt")
            .increment(proxies.len() as u64);
    }

    let path = config
        .output
        .path
        .canonicalize()
        .unwrap_or_else(move |_| config.output.path.clone());
    if is_docker().await {
        tracing::info!(
            "Proxies have been saved to ./out ({} in container)",
            path.display()
        );
    } else {
        tracing::info!("Proxies have been saved to {}", path.display());
    }

    Ok(())
}

fn filter_proxies(
    config: &Config,
    proxies: &mut Vec<Proxy>,
    maybe_asn_db: Option<&maxminddb::Reader<maxminddb::Mmap>>,
    maybe_geo_db: Option<&maxminddb::Reader<maxminddb::Mmap>>,
) -> crate::Result<()> {
    proxies.retain(|proxy| {
        // Anonymity filter
        if let Some(min_anon) = config.output.filters.min_anonymity {
            match proxy.anonymity {
                Some(proxy_anon) if proxy_anon >= min_anon => {}
                _ => return false,
            }
        }

        // Latency filter
        if let Some(max_lat) = config.output.filters.max_latency {
            match proxy.timeout {
                Some(proxy_lat) if proxy_lat <= max_lat => {}
                _ => return false,
            }
        }

        // Country code filter
        if let Some(ref only_cc) = config.output.filters.only_cc {
            let (_, geo) = lookup_metadata(
                proxy,
                maybe_asn_db,
                maybe_geo_db,
            )
            .unwrap_or((None, None));

            match geo.and_then(|g| g.country).and_then(|c| c.iso_code) {
                Some(cc) if only_cc.contains(&cc.to_string()) => {}
                _ => return false,
            }
        }

        true
    });

    Ok(())
}

fn lookup_metadata<'a>(
    proxy: &Proxy,
    maybe_asn_db: Option<&'a maxminddb::Reader<maxminddb::Mmap>>,
    maybe_geo_db: Option<&'a maxminddb::Reader<maxminddb::Mmap>>,
) -> crate::Result<(
    Option<maxminddb::geoip2::Asn<'a>>,
    Option<maxminddb::geoip2::City<'a>>,
)> {
    let exit_ip_addr: Option<IpAddr> = proxy
        .exit_ip
        .as_ref()
        .and_then(|ip| ip.parse().ok());

    let asn = if let (Some(asn_db), Some(ip)) = (maybe_asn_db, exit_ip_addr) {
        asn_db.lookup::<maxminddb::geoip2::Asn<'_>>(ip).ok().flatten()
    } else {
        None
    };

    let geolocation = if let (Some(geo_db), Some(ip)) = (maybe_geo_db, exit_ip_addr) {
        geo_db.lookup::<maxminddb::geoip2::City<'_>>(ip).ok().flatten()
    } else {
        None
    };

    Ok((asn, geolocation))
}

async fn write_proxy_list<'a, I>(
    path: &std::path::Path,
    proxies: I,
    include_protocol: bool,
    format: Option<&str>,
    maybe_asn_db: Option<&'a maxminddb::Reader<maxminddb::Mmap>>,
    maybe_geo_db: Option<&'a maxminddb::Reader<maxminddb::Mmap>>,
    hostnames: &HashMap<String, String>,
) -> crate::Result<()>
where
    I: IntoIterator<Item = &'a Proxy>,
{
    use tokio::io::AsyncWriteExt as _;

    let file = tokio::fs::File::create(path)
        .await
        .wrap_err_with(|| format!("failed to create file {}", path.display()))?;
    let mut writer = tokio::io::BufWriter::new(file);

    for proxy in proxies {
        let s = if let Some(format) = format {
            let (asn, geo) = lookup_metadata(proxy, maybe_asn_db, maybe_geo_db)?;
            let hostname = proxy.exit_ip.as_ref().and_then(|ip| hostnames.get(ip).cloned());
            render_template(format, proxy, include_protocol, asn.as_ref(), geo.as_ref(), hostname)
        } else {
            proxy.to_string(include_protocol).to_string()
        };
        writer
            .write_all(s.as_bytes())
            .await
            .wrap_err("failed to write to file")?;
        writer.write_all(b"\n").await.wrap_err("failed to write to file")?;
    }
    writer.flush().await.wrap_err("failed to flush file")?;
    Ok(())
}

fn render_template(
    template: &str,
    proxy: &Proxy,
    include_protocol: bool,
    asn: Option<&maxminddb::geoip2::Asn<'_>>,
    geo: Option<&maxminddb::geoip2::City<'_>>,
    hostname: Option<String>,
) -> String {
    let mut result = template.to_string();

    // Basic variables
    result = result.replace("{{proxy}}", &proxy.to_string(include_protocol));
    result = result.replace("{{protocol}}", proxy.protocol.as_str());
    result = result.replace("{{host}}", &proxy.host);
    result = result.replace("{{port}}", &proxy.port.to_string());
    result = result.replace("{{ip}}", proxy.exit_ip.as_deref().unwrap_or(""));
    result = result.replace(
        "{{duration}}",
        &proxy
            .timeout
            .map(|d| format!("{}ms", d.as_millis()))
            .unwrap_or_default(),
    );

    // Metadata variables
    if let Some(asn) = asn {
        result = result.replace(
            "{{org}}",
            asn.autonomous_system_organization.as_deref().unwrap_or(""),
        );
    } else {
        result = result.replace("{{org}}", "");
    }

    if let Some(geo) = geo {
        result = result.replace(
            "{{country}}",
            geo.country
                .as_ref()
                .and_then(|c| c.iso_code)
                .unwrap_or(""),
        );
        result = result.replace(
            "{{city}}",
            geo.city
                .as_ref()
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en"))
                .copied()
                .unwrap_or(""),
        );
        result = result.replace(
            "{{region}}",
            geo.subdivisions
                .as_ref()
                .and_then(|s| s.first())
                .and_then(|s| s.iso_code)
                .unwrap_or(""),
        );
        result = result.replace(
            "{{timezone}}",
            geo.location
                .as_ref()
                .and_then(|l| l.time_zone)
                .unwrap_or(""),
        );
        let loc = geo
            .location
            .as_ref()
            .and_then(|l| {
                if let (Some(lat), Some(long)) = (l.latitude, l.longitude) {
                    Some(format!("{lat},{long}"))
                } else {
                    None
                }
            })
            .unwrap_or_default();
        result = result.replace("{{loc}}", &loc);
    } else {
        for var in &["{{country}}", "{{city}}", "{{region}}", "{{timezone}}", "{{loc}}"] {
            result = result.replace(var, "");
        }
    }

    // Hostname, Anonymity and Score
    result = result.replace("{{hostname}}", &hostname.unwrap_or_default());
    result = result.replace(
        "{{anonymity}}",
        proxy.anonymity.map(|a| a.as_str()).unwrap_or_default(),
    );
    result = result.replace(
        "{{score}}",
        proxy
            .score
            .map(|s| s.to_string())
            .as_deref()
            .unwrap_or_default(),
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::{Proxy, ProxyType};
    use std::time::Duration;

    #[test]
    fn test_render_template_basic() {
        let proxy = Proxy {
            protocol: ProxyType::Http,
            host: "1.2.3.4".into(),
            port: 8080,
            username: None,
            password: None,
            timeout: Some(Duration::from_millis(150)),
            exit_ip: Some("5.6.7.8".into()),
            anonymity: None,
            score: None,
        };

        let template = "{{protocol}}://{{host}}:{{port}} [{{ip}}] ({{duration}})";
        let rendered = render_template(template, &proxy, true, None, None, None);
        assert_eq!(rendered, "http://1.2.3.4:8080 [5.6.7.8] (150ms)");
    }

    #[test]
    fn test_render_template_hostname() {
        let proxy = Proxy {
            protocol: ProxyType::Http,
            host: "1.2.3.4".into(),
            port: 8080,
            username: None,
            password: None,
            timeout: Some(Duration::from_millis(150)),
            exit_ip: Some("5.6.7.8".into()),
            anonymity: None,
            score: None,
        };

        let template = "{{proxy}} - {{hostname}}";
        let rendered = render_template(template, &proxy, true, None, None, Some("example.com".into()));
        assert_eq!(rendered, "http://1.2.3.4:8080 - example.com");
    }

    #[test]
    fn test_render_template_anonymity() {
        let proxy = Proxy {
            protocol: ProxyType::Http,
            host: "1.2.3.4".into(),
            port: 8080,
            username: None,
            password: None,
            timeout: Some(Duration::from_millis(150)),
            exit_ip: Some("5.6.7.8".into()),
            anonymity: Some(crate::proxy::AnonymityLevel::Elite),
            score: None,
        };

        let template = "{{proxy}} - {{anonymity}}";
        let rendered = render_template(template, &proxy, true, None, None, None);
        assert_eq!(rendered, "http://1.2.3.4:8080 - elite");
    }

    #[test]
    fn test_filter_proxies_anonymity() {
        use crate::proxy::AnonymityLevel;
        use crate::config::{Config, ScrapingConfig, CheckingConfig, OutputConfig, TxtOutputConfig, JsonOutputConfig, OutputFilters, ServerConfig};
        use std::path::PathBuf;

        let mut proxies = vec![
            Proxy {
                protocol: ProxyType::Http,
                host: "1.1.1.1".into(),
                port: 80,
                username: None,
                password: None,
                timeout: None,
                exit_ip: None,
                anonymity: Some(AnonymityLevel::Elite),
                score: None,
            },
            Proxy {
                protocol: ProxyType::Http,
                host: "2.2.2.2".into(),
                port: 80,
                username: None,
                password: None,
                timeout: None,
                exit_ip: None,
                anonymity: Some(AnonymityLevel::Anonymous),
                score: None,
            },
        ];

        let config = Config {
            debug: false,
            scraping: ScrapingConfig {
                max_proxies_per_source: 0,
                timeout: Duration::ZERO,
                connect_timeout: Duration::ZERO,
                proxy: None,
                user_agent: "".into(),
                sources: HashMap::default(),
            },
            checking: CheckingConfig {
                check_url: None,
                max_concurrent_checks: 0,
                timeout: Duration::ZERO,
                connect_timeout: Duration::ZERO,
                user_agent: "".into(),
            },
            output: OutputConfig {
                path: PathBuf::from("."),
                sort_by_speed: false,
                txt: TxtOutputConfig { enabled: true, format: None },
                json: JsonOutputConfig { enabled: false, include_asn: false, include_geolocation: false },
                rank: false,
                top: None,
                profile: None,
                filters: OutputFilters {
                    min_anonymity: Some(AnonymityLevel::Elite),
                    max_latency: None,
                    only_cc: None,
                },
            },
            server: ServerConfig {
                enabled: false,
                bind_addr: "127.0.0.1:0".parse().unwrap(),
                tor_isolation: false,
                auth: None,
                rotation_method: "".into(),
                rotate_after_requests: 0,
                rotate_on_error: false,
                remove_on_error: false,
                max_errors: None,
                max_redirs: None,
                max_retries: None,
                country_filter: None,
                sync: false,
                verbose: false,
                timeout: Duration::ZERO,
                output: None,
            },
        };

        filter_proxies(&config, &mut proxies, None, None).unwrap();

        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].host, "1.1.1.1");
    }

    #[test]
    fn test_filter_proxies_latency() {
        use crate::config::{Config, ScrapingConfig, CheckingConfig, OutputConfig, TxtOutputConfig, JsonOutputConfig, OutputFilters, ServerConfig};
        use std::path::PathBuf;

        let mut proxies = vec![
            Proxy {
                protocol: ProxyType::Http,
                host: "1.1.1.1".into(),
                port: 80,
                username: None,
                password: None,
                timeout: Some(Duration::from_millis(100)),
                exit_ip: None,
                anonymity: None,
                score: None,
            },
            Proxy {
                protocol: ProxyType::Http,
                host: "2.2.2.2".into(),
                port: 80,
                username: None,
                password: None,
                timeout: Some(Duration::from_millis(500)),
                exit_ip: None,
                anonymity: None,
                score: None,
            },
        ];

        let config = Config {
            debug: false,
            scraping: ScrapingConfig {
                max_proxies_per_source: 0,
                timeout: Duration::ZERO,
                connect_timeout: Duration::ZERO,
                proxy: None,
                user_agent: "".into(),
                sources: HashMap::default(),
            },
            checking: CheckingConfig {
                check_url: None,
                max_concurrent_checks: 0,
                timeout: Duration::ZERO,
                connect_timeout: Duration::ZERO,
                user_agent: "".into(),
            },
            output: OutputConfig {
                path: PathBuf::from("."),
                sort_by_speed: false,
                txt: TxtOutputConfig { enabled: true, format: None },
                json: JsonOutputConfig { enabled: false, include_asn: false, include_geolocation: false },
                rank: false,
                top: None,
                profile: None,
                filters: OutputFilters {
                    min_anonymity: None,
                    max_latency: Some(Duration::from_millis(300)),
                    only_cc: None,
                },
            },
            server: ServerConfig {
                enabled: false,
                bind_addr: "127.0.0.1:0".parse().unwrap(),
                tor_isolation: false,
                auth: None,
                rotation_method: "".into(),
                rotate_after_requests: 0,
                rotate_on_error: false,
                remove_on_error: false,
                max_errors: None,
                max_redirs: None,
                max_retries: None,
                country_filter: None,
                sync: false,
                verbose: false,
                timeout: Duration::ZERO,
                output: None,
            },
        };

        filter_proxies(&config, &mut proxies, None, None).unwrap();

        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].host, "1.1.1.1");
    }

    #[test]
    fn test_profile_expansion() {
        use crate::config::{OutputFilters, Profile};
        use crate::proxy::AnonymityLevel;
        
        let mut filters = OutputFilters::default();
        Profile::Scraping.apply(&mut filters);
        assert_eq!(filters.min_anonymity, Some(AnonymityLevel::Anonymous));
        assert_eq!(filters.max_latency, Some(Duration::from_secs(5)));
        
        let mut filters = OutputFilters::default();
        Profile::Stealth.apply(&mut filters);
        assert_eq!(filters.min_anonymity, Some(AnonymityLevel::Elite));
        assert_eq!(filters.max_latency, Some(Duration::from_secs(5)));
        
        let mut filters = OutputFilters::default();
        Profile::Speed.apply(&mut filters);
        assert_eq!(filters.max_latency, Some(Duration::from_secs(1)));
    }

    #[test]
    fn test_ranking_logic() {
        let mut proxies = vec![
            Proxy {
                protocol: ProxyType::Http,
                host: "1.1.1.1".into(),
                port: 80,
                username: None,
                password: None,
                timeout: Some(Duration::from_millis(500)),
                exit_ip: None,
                anonymity: None,
                score: Some(50),
            },
            Proxy {
                protocol: ProxyType::Http,
                host: "2.2.2.2".into(),
                port: 80,
                username: None,
                password: None,
                timeout: Some(Duration::from_millis(100)),
                exit_ip: None,
                anonymity: None,
                score: Some(90),
            },
        ];

        // Sort descending by score
        proxies.sort_by(|a, b| b.score.unwrap_or(0).cmp(&a.score.unwrap_or(0)));
        
        assert_eq!(proxies[0].host, "2.2.2.2");
        assert_eq!(proxies[1].host, "1.1.1.1");
    }
}
