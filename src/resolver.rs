//! Custom DNS resolver using `hickory-resolver`.

use std::{net::SocketAddr, sync::Arc};

/// A DNS resolver that uses `hickory-resolver` for asynchronous DNS lookups.
pub struct HickoryDnsResolver(Arc<hickory_resolver::TokioResolver>);

impl HickoryDnsResolver {
    /// Creates a new `HickoryDnsResolver` with default settings.
    pub fn new() -> Self {
        let mut builder = hickory_resolver::TokioResolver::builder_tokio()
            .unwrap_or_else(|_| {
                hickory_resolver::TokioResolver::builder_with_config(
                hickory_resolver::config::ResolverConfig::cloudflare(),
                hickory_resolver::name_server::TokioConnectionProvider::default(
                ),
            )
            });
        builder.options_mut().ip_strategy =
            hickory_resolver::config::LookupIpStrategy::Ipv4AndIpv6;
        Self(Arc::new(builder.build()))
    }

    /// Performs a reverse DNS lookup for the given IP address.
    pub async fn reverse_lookup(
        &self,
        ip: std::net::IpAddr,
    ) -> crate::Result<Option<String>> {
        let resolver = Arc::clone(&self.0);
        let lookup = resolver.reverse_lookup(ip).await?;
        Ok(lookup.iter().next().map(|name| name.to_utf8()))
    }
}

impl Default for HickoryDnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl reqwest::dns::Resolve for HickoryDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let resolver = Arc::clone(&self.0);
        Box::pin(async move {
            let lookup = resolver.lookup_ip(name.as_str()).await?;
            drop(resolver);
            let addrs: reqwest::dns::Addrs = Box::new(
                lookup.into_iter().map(|ip_addr| SocketAddr::new(ip_addr, 0)),
            );
            Ok(addrs)
        })
    }
}
