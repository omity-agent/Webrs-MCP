use crate::{Result, config::SsrfConfig, error::AppError};
use core::net::{IpAddr, SocketAddr};
use http_acl::HttpAcl;
use std::sync::LazyLock;
use tokio::net::lookup_host;
use url::{Host, Url};
static PUBLIC_NETWORK_ACL: LazyLock<HttpAcl> =
    LazyLock::new(|| HttpAcl::builder().ip_acl_default(true).build());
#[derive(Clone, Debug)]
pub struct SsrfGuard {
    config: SsrfConfig,
}
impl SsrfGuard {
    #[inline]
    #[must_use]
    pub const fn new(config: SsrfConfig) -> Self {
        Self { config }
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "URL validation may perform DNS lookup and is not an inline candidate."
    )]
    pub async fn validate_url(&self, url: &Url) -> Result<()> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AppError::client("URL must use HTTP or HTTPS."));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(AppError::client("URL credentials are not allowed."));
        }
        let Some(host) = url.host() else {
            return Err(AppError::client("URL must include a host."));
        };
        self.validate_host(host, url.port_or_known_default()).await
    }
    async fn validate_host(&self, host: Host<&str>, port: Option<u16>) -> Result<()> {
        match host {
            Host::Ipv4(address) => self.validate_ip(IpAddr::V4(address)),
            Host::Ipv6(address) => self.validate_ip(IpAddr::V6(address)),
            Host::Domain(domain) => self.validate_domain(domain, port).await,
        }
    }
    async fn validate_domain(&self, domain: &str, port: Option<u16>) -> Result<()> {
        let normalized = domain.trim_end_matches('.').to_ascii_lowercase();
        if self.config.block_local_hostnames && is_local_hostname(&normalized) {
            return Err(AppError::client("URL host is blocked by SSRF protection."));
        }
        if !self.config.block_private_networks {
            return Ok(());
        }
        self.resolve_allowed_domain(&normalized, port.unwrap_or(443))
            .await
            .map(|_addresses| ())
    }
    fn validate_ip(&self, address: IpAddr) -> Result<()> {
        if !self.config.block_private_networks || is_public_ip(address) {
            return Ok(());
        }
        Err(AppError::client(
            "URL resolves to a blocked network address.",
        ))
    }
    pub(crate) async fn resolve_allowed_domain(
        &self,
        domain: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>> {
        let normalized = domain.trim_end_matches('.').to_ascii_lowercase();
        if self.config.block_local_hostnames && is_local_hostname(&normalized) {
            return Err(AppError::client("URL host is blocked by SSRF protection."));
        }
        let addresses = lookup_host((normalized.as_str(), port))
            .await
            .map_err(|_error| AppError::client("URL host could not be resolved."))?;
        self.validate_resolved_addresses(addresses)
    }
    pub(crate) fn validate_resolved_addresses<I>(&self, addresses: I) -> Result<Vec<SocketAddr>>
    where
        I: IntoIterator<Item = SocketAddr>,
    {
        let mut allowed = Vec::new();
        for socket in addresses {
            self.validate_ip(socket.ip())?;
            allowed.push(socket);
        }
        if allowed.is_empty() {
            return Err(AppError::client("URL host did not resolve to any address."));
        }
        Ok(allowed)
    }
}
#[inline]
#[must_use]
pub fn is_public_ip(address: IpAddr) -> bool {
    !address.is_multicast() && PUBLIC_NETWORK_ACL.is_ip_allowed(&address).is_allowed()
}
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "Hostnames are already lowercased before local suffix checks."
)]
fn is_local_hostname(host: &str) -> bool {
    matches!(
        host,
        "localhost" | "localhost.localdomain" | "ip6-localhost" | "ip6-loopback"
    ) || host.ends_with(".localhost")
        || host.ends_with(".local")
}
