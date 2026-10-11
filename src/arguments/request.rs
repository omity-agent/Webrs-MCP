use crate::arguments::{
    aliases::{self, FieldSpec},
    field::normalize_field,
    singleton::single_valid_string,
    support::parse_json_container,
};
use crate::warnings::Warnings;
use alloc::collections::BTreeMap;
use sonic_rs::{JsonContainerTrait as _, Value};
pub type RequestMap = BTreeMap<String, Value>;
#[derive(Clone, Copy)]
pub(crate) struct RequestNormalizer {
    fields: &'static [FieldSpec],
    primary_field: Option<&'static str>,
}
impl RequestNormalizer {
    #[inline]
    #[must_use]
    pub(crate) const fn new(
        fields: &'static [FieldSpec],
        primary_field: Option<&'static str>,
    ) -> Self {
        Self {
            fields,
            primary_field,
        }
    }
    pub(crate) fn normalize_value(
        self,
        value: Value,
        warnings: &mut Warnings,
        path: &str,
    ) -> RequestMap {
        let parsed = parse_json_container(value);
        let Some(object) = parsed.as_object() else {
            return self
                .singleton_request(&parsed, warnings, path)
                .unwrap_or_default();
        };
        self.normalize_object(&parsed, object, warnings, path)
    }
    pub(crate) fn normalize_object(
        self,
        value: &Value,
        object: &sonic_rs::Object,
        warnings: &mut Warnings,
        path: &str,
    ) -> RequestMap {
        if !self.has_named_field(object)
            && let Some(request) = self.singleton_request(value, warnings, path)
        {
            return request;
        }
        let mut normalized = RequestMap::new();
        for (raw_key, raw_item) in object {
            self.insert_field(&mut normalized, raw_key, raw_item, warnings, path);
        }
        normalized
    }
    pub(crate) fn singleton_request(
        self,
        value: &Value,
        warnings: &mut Warnings,
        path: &str,
    ) -> Option<RequestMap> {
        let field = self.primary_field?;
        let text = single_valid_string(value)?;
        let mut request = RequestMap::new();
        request.insert(
            field.to_owned(),
            normalize_field(
                field,
                Value::from(text.as_str()),
                warnings,
                &format!("{path}.{field}"),
            ),
        );
        Some(request)
    }
    fn insert_field(
        self,
        normalized: &mut RequestMap,
        raw_key: &str,
        raw_item: &Value,
        warnings: &mut Warnings,
        path: &str,
    ) {
        let Some(canonical) = aliases::canonical_field(self.fields, raw_key) else {
            warnings.push_unique(format!("ignored unrecognized field \"{path}.{raw_key}\""));
            return;
        };
        warn_alias(canonical, raw_key, warnings);
        warn_duplicate(normalized, canonical, warnings);
        normalized.insert(
            canonical.to_owned(),
            normalize_field(
                canonical,
                raw_item.clone(),
                warnings,
                &format!("{path}.{canonical}"),
            ),
        );
    }
    fn has_named_field(self, object: &sonic_rs::Object) -> bool {
        object
            .iter()
            .any(|(key, _value)| aliases::canonical_field(self.fields, key).is_some())
    }
}
fn warn_alias(canonical: &str, raw_key: &str, warnings: &mut Warnings) {
    if canonical != raw_key {
        warnings.push_unique(format!("use \"{canonical}\" instead of \"{raw_key}\""));
    }
}
fn warn_duplicate(normalized: &RequestMap, canonical: &str, warnings: &mut Warnings) {
    if normalized.contains_key(canonical) {
        warnings.push_unique(format!(
            "multiple aliases for \"{canonical}\" were provided; the last value was used"
        ));
    }
}
