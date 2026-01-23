use std::{net::SocketAddr, sync::Arc};

pub struct HickoryDnsResolver(Arc<hickory_resolver::TokioResolver>);

impl HickoryDnsResolver {
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
