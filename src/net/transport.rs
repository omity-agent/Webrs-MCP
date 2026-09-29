use crate::{
    Result,
    error::AppError,
    net::{SsrfGuard, resolver::GuardedResolver},
};
use alloc::sync::Arc;
use reqwest::{Client, header::HeaderValue, redirect::Policy};
pub(super) fn clients(
    guard: &SsrfGuard,
    user_agent: &HeaderValue,
    max_redirects: usize,
) -> Result<(Client, Client)> {
    let build = |policy| {
        Client::builder()
            .dns_resolver(Arc::new(GuardedResolver::new(guard.clone())))
            .redirect(policy)
            .referer(false)
            .no_proxy()
            .user_agent(user_agent.clone())
            .build()
            .map_err(|error| {
                AppError::internal(format!("failed to build guarded HTTP client: {error}"))
            })
    };
    Ok((
        build(redirects(guard.clone(), max_redirects))?,
        build(Policy::none())?,
    ))
}
fn redirects(guard: SsrfGuard, max_redirects: usize) -> Policy {
    Policy::custom(move |attempt| {
        if let Err(error) = guard.check_url(attempt.url()) {
            return attempt.error(error);
        }
        if attempt.previous().len() > max_redirects {
            return attempt.error(AppError::client("Too many redirects while fetching URL."));
        }
        attempt.follow()
    })
}
