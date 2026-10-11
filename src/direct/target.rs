#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Direct fetch response formats are closed protocol cases handled exhaustively."
)]
pub enum ResponseFormat {
    Markdown,
    Text,
    RustdocJson,
    MediaWikiApi,
    PackageRegistryJson,
    StackOverflowQuestionJson,
}
#[derive(Clone, Debug)]
pub struct DirectFetchTarget {
    pub original_url: String,
    pub request_url: String,
    pub similarity_probe_url: Option<String>,
    pub response_format: ResponseFormat,
    pub json_fields_first: Vec<String>,
    pub json_fields_last: Vec<String>,
}
impl DirectFetchTarget {
    #[inline]
    #[must_use]
    pub fn text<OriginalUrl, RequestUrl>(original_url: OriginalUrl, request_url: RequestUrl) -> Self
    where
        OriginalUrl: Into<String>,
        RequestUrl: Into<String>,
    {
        Self {
            original_url: original_url.into(),
            request_url: request_url.into(),
            similarity_probe_url: None,
            response_format: ResponseFormat::Text,
            json_fields_first: Vec::new(),
            json_fields_last: Vec::new(),
        }
    }
    #[inline]
    #[must_use]
    pub fn markdown<OriginalUrl, RequestUrl>(
        original_url: OriginalUrl,
        request_url: RequestUrl,
    ) -> Self
    where
        OriginalUrl: Into<String>,
        RequestUrl: Into<String>,
    {
        let mut target = Self::text(original_url, request_url);
        target.response_format = ResponseFormat::Markdown;
        target
    }
    #[inline]
    #[must_use]
    pub fn rustdoc_json<OriginalUrl, RequestUrl>(
        original_url: OriginalUrl,
        request_url: RequestUrl,
    ) -> Self
    where
        OriginalUrl: Into<String>,
        RequestUrl: Into<String>,
    {
        let mut target = Self::text(original_url, request_url);
        target.response_format = ResponseFormat::RustdocJson;
        target
    }
    #[inline]
    #[must_use]
    pub fn package(
        original_url: &str,
        request_url: String,
        fields_first: Vec<String>,
        fields_last: Vec<String>,
    ) -> Self {
        let mut target = Self::text(original_url, request_url);
        target.response_format = ResponseFormat::PackageRegistryJson;
        target.json_fields_first = fields_first;
        target.json_fields_last = fields_last;
        target
    }
    #[inline]
    #[must_use]
    pub fn mediawiki(original_url: &str, request_url: String) -> Self {
        let mut target = Self::text(original_url, request_url);
        target.response_format = ResponseFormat::MediaWikiApi;
        target
    }
    #[inline]
    #[must_use]
    pub fn stack_overflow_question(original_url: &str, request_url: String) -> Self {
        let mut target = Self::text(original_url, request_url);
        target.response_format = ResponseFormat::StackOverflowQuestionJson;
        target
    }
}
