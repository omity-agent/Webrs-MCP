pub const SEARCH_FIELDS: &[FieldSpec] = &[
    FieldSpec::new(
        "q",
        &[
            "query",
            "queries",
            "term",
            "terms",
            "search_term",
            "search_terms",
        ],
    ),
    FieldSpec::new("recency", &["recencies"]),
    FieldSpec::new("domains", &["domain", "urls", "url"]),
    FieldSpec::new("category", &["categories", "type", "class"]),
];
pub const OPEN_FIELDS: &[FieldSpec] = &[
    FieldSpec::new("url", &["urls", "domain", "domains"]),
    FieldSpec::new("chunk", &["chunks", "block", "blocks", "piece", "pieces"]),
];
pub const FIND_FIELDS: &[FieldSpec] = &[
    FieldSpec::new("url", &["urls", "domain", "domains"]),
    FieldSpec::new("pattern", &["patterns"]),
    FieldSpec::new("snippet_tokens", &["snippet_token", "snippet", "snippets"]),
];
#[derive(Clone, Copy, Debug)]
pub struct FieldSpec {
    canonical: &'static str,
    aliases: &'static [&'static str],
}
impl FieldSpec {
    #[inline]
    #[must_use]
    pub const fn new(canonical: &'static str, aliases: &'static [&'static str]) -> Self {
        Self { canonical, aliases }
    }
    #[inline]
    #[must_use]
    pub fn matches(self, raw: &str) -> bool {
        raw.eq_ignore_ascii_case(self.canonical)
            || self
                .aliases
                .iter()
                .any(|alias| raw.eq_ignore_ascii_case(alias))
    }
    #[inline]
    #[must_use]
    pub const fn canonical(self) -> &'static str {
        self.canonical
    }
}
#[inline]
#[must_use]
pub fn canonical_field<'field>(fields: &'field [FieldSpec], raw: &str) -> Option<&'field str> {
    fields
        .iter()
        .find(|field| field.matches(raw))
        .map(|field| field.canonical())
}
#[inline]
#[must_use]
pub fn canonical_category(raw: &str) -> Option<&'static str> {
    let lowered = raw.trim().to_ascii_lowercase();
    let normalized = lowered.replace(['_', '-'], " ");
    match normalized.as_str() {
        "company" | "companies" => Some("company"),
        "research paper" | "research papers" => Some("research paper"),
        "news" => Some("news"),
        "pdf" | "pdfs" => Some("pdf"),
        "personal site" | "personal sites" => Some("personal site"),
        "financial report" | "financial reports" => Some("financial report"),
        "people" | "person" => Some("people"),
        _ => None,
    }
}
#[inline]
#[must_use]
pub fn looks_like_request<Keys>(fields: &[FieldSpec], keys: Keys) -> bool
where
    Keys: Iterator,
    Keys::Item: AsRef<str>,
{
    keys.into_iter()
        .any(|key| canonical_field(fields, key.as_ref()).is_some())
}
