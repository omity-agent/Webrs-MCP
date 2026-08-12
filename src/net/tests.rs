use crate::{
    config::SsrfConfig,
    net::{
        resolver::GuardedResolver,
        ssrf::{SsrfGuard, is_public_ip},
    },
};
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use reqwest::dns::Name;
#[test]
fn private_and_loopback_addresses_are_not_public() {
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
}
#[test]
fn documentation_public_example_address_is_allowed() {
    assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
}
#[test]
fn unique_local_ipv6_address_is_not_public() {
    assert!(!is_public_ip(IpAddr::V6(Ipv6Addr::new(
        0xfc00, 0, 0, 0, 0, 0, 0, 1
    ))));
}
#[test]
fn discard_only_ipv6_address_is_not_public() {
    assert!(!is_public_ip(IpAddr::V6(Ipv6Addr::new(
        0x0100, 0, 0, 0, 0, 0, 0, 1
    ))));
}
#[test]
fn ssrf_guard_rejects_private_dns_answers() {
    let guard = blocked_network_guard();
    let blocked = [SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80)];
    let result = guard.validate_resolved_addresses(blocked);
    match result {
        Ok(addresses) => panic!("private DNS answers should be rejected: {addresses:?}"),
        Err(error) => assert_eq!(
            error.client_message(),
            "URL resolves to a blocked network address."
        ),
    }
}
#[test]
fn ssrf_guard_accepts_public_dns_answers() {
    let guard = blocked_network_guard();
    let public = [SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        80,
    )];
    let result = guard.validate_resolved_addresses(public);
    match result {
        Ok(addresses) => assert_eq!(addresses.len(), 1),
        Err(error) => panic!("public DNS answers should be accepted: {error}"),
    }
}
#[tokio::test]
async fn guarded_resolver_rejects_localhost_before_connection() {
    let resolver = GuardedResolver::new(blocked_network_guard());
    let name = localhost_name();
    let result = reqwest::dns::Resolve::resolve(&resolver, name).await;
    match result {
        Ok(_addresses) => panic!("localhost should be rejected before connection"),
        Err(error) => assert_eq!(error.to_string(), "URL host is blocked by SSRF protection."),
    }
}
fn blocked_network_guard() -> SsrfGuard {
    SsrfGuard::new(blocked_network_config())
}
const fn blocked_network_config() -> SsrfConfig {
    SsrfConfig {
        block_private_networks: true,
        block_local_hostnames: true,
    }
}
fn localhost_name() -> Name {
    match "localhost".parse::<Name>() {
        Ok(name) => name,
        Err(error) => panic!("localhost should be a valid DNS name: {error}"),
    }
}
