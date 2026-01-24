//! Proxy checking module.
//!
//! This module provides the logic for validating a large number of proxies
//! concurrently using a worker pool pattern.

use std::sync::Arc;

use color_eyre::eyre::OptionExt as _;

#[cfg(feature = "tui")]
use crate::event::{AppEvent, Event};
use crate::{config::Config, proxy::Proxy, utils::pretty_error};

/// Validates all given proxies concurrently.
///
/// This function uses a worker pool to check the provided proxies.
/// Only proxies that pass the check (i.e., meet the connectivity and anonymity
/// requirements) are returned.
///
/// # Errors
///
/// Returns an error if the worker pool fails or if tasks are cancelled unexpectedly.
pub async fn check_all<R: reqwest::dns::Resolve + 'static>(
    config: Arc<Config>,
    dns_resolver: Arc<R>,
    proxies: Vec<Proxy>,
    token: tokio_util::sync::CancellationToken,
    #[cfg(feature = "tui")] tx: tokio::sync::mpsc::UnboundedSender<Event>,
) -> crate::Result<Vec<Proxy>> {
    if config.checking.check_url.is_none() {
        return Ok(proxies);
    }

    let workers_count =
        config.checking.max_concurrent_checks.min(proxies.len());
    if workers_count == 0 {
        return Ok(Vec::new());
    }

    #[cfg(not(feature = "tui"))]
    tracing::info!("Started checking {} proxies", proxies.len());

    let queue = Arc::new(parking_lot::Mutex::new(proxies));
    let checked_proxies = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let mut join_set = tokio::task::JoinSet::<()>::new();
    for _ in 0..workers_count {
        let queue = Arc::clone(&queue);
        let config = Arc::clone(&config);
        let dns_resolver = Arc::clone(&dns_resolver);
        let checked_proxies = Arc::clone(&checked_proxies);
        let token = token.clone();
        #[cfg(feature = "tui")]
        let tx = tx.clone();

        // Spawn a fixed number of workers to process the queue
        join_set.spawn(async move {
            tokio::select! {
                biased;
                res = async move {
                    loop {
                        // Pop from the shared queue until it's empty
                        let Some(mut proxy) = queue.lock().pop() else {
                            break;
                        };
                        let check_result = proxy.check(&config, Arc::clone(&dns_resolver)).await;
                        
                        #[cfg(feature = "tui")]
                        drop(tx.send(Event::App(AppEvent::ProxyChecked(proxy.protocol))));

                        metrics::counter!("proxies_checked_total").increment(1);

                        match check_result {
                            Ok(()) => {
                                #[cfg(feature = "tui")]
                                drop(tx.send(Event::App(AppEvent::ProxyWorking(proxy.protocol))));

                                metrics::counter!("proxies_working_total", "protocol" => proxy.protocol.as_str()).increment(1);
                                if let Some(duration) = proxy.timeout {
                                    metrics::histogram!("proxy_check_duration_seconds", "protocol" => proxy.protocol.as_str()).record(duration.as_secs_f64());
                                }

                                // Store successful proxies in the result vector
                                checked_proxies.lock().push(proxy);
                            }
                            Err(e) if tracing::event_enabled!(tracing::Level::DEBUG) => {
                                tracing::debug!(
                                    "{}: {}",
                                    proxy.to_string(true),
                                    pretty_error(&e)
                                );
                            }
                            Err(_) => {}
                        }
                    }
                } => res,
                () = token.cancelled() => (),
            }
        });
    }

    drop(queue);
    drop(config);
    drop(dns_resolver);
    drop(token);
    #[cfg(feature = "tui")]
    drop(tx);

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(()) => {}
            Err(e) if e.is_panic() => {
                tracing::error!(
                    "Proxy checking task panicked: {}",
                    pretty_error(&e.into())
                );
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    drop(join_set);

    Ok(Arc::into_inner(checked_proxies)
        .ok_or_eyre("failed to unwrap Arc")?
        .into_inner())
}
