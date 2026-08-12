use crate::{
    config::SsrfConfig,
    direct::{DirectFetchTarget, fetch::request_headers},
    net::{SecureHttpClient, SsrfGuard},
};
#[test]
fn direct_request_headers_do_not_send_range() {
    let client = SecureHttpClient::new(
        0,
        1,
        "web-rs-test",
        SsrfGuard::new(SsrfConfig {
            block_private_networks: false,
            block_local_hostnames: false,
        }),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let target = DirectFetchTarget::text(
        "https://example.com/page",
        "https://example.com/page.txt".to_owned(),
    );
    let headers = request_headers(&client, &target).unwrap_or_else(|error| panic!("{error}"));
    assert!(!headers.contains_key(reqwest::header::RANGE));
}
