mod dispatch;
mod exa;
mod filters;
mod octen;
pub type SearchBatch = dispatch::SearchBatch;
pub type SearchClient = dispatch::SearchClient;
fn normalize_highlight(highlight: &str) -> String {
    let mut normalized = String::with_capacity(highlight.len());
    let mut consecutive_line_feeds: u8 = 0;
    for character in highlight.chars() {
        if character == '\n' {
            if consecutive_line_feeds < 2 {
                consecutive_line_feeds += 1;
                normalized.push(character);
            }
        } else {
            consecutive_line_feeds = 0;
            normalized.push(character);
        }
    }
    normalized
}
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
