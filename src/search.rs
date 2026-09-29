mod dispatch;
mod exa;
mod filters;
mod octen;
pub type SearchBatch = dispatch::SearchBatch;
pub type SearchClient = dispatch::SearchClient;
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SearchCredentials {
    Exa(String),
    Octen(String),
}
impl SearchCredentials {
    #[inline]
    #[must_use]
    pub const fn provider_name(&self) -> &'static str {
        match *self {
            Self::Exa(_) => "Exa",
            Self::Octen(_) => "Octen",
        }
    }
    #[inline]
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowing the enum variants keeps API keys owned by the credentials."
    )]
    pub fn api_key(&self) -> &str {
        match self {
            Self::Exa(key) | Self::Octen(key) => key,
        }
    }
}
