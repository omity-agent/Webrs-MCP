use crate::config::DirectFetchConfig;
use percent_encoding::percent_decode_str;
use serde::{Serialize, Serializer, ser::SerializeMap as _};
use sonic_rs::JsonContainerTrait as _;
use url::Url;
#[derive(Debug)]
pub struct PackageRegistryTarget {
    pub request_url: String,
    pub json_fields_first: Vec<String>,
    pub json_fields_last: Vec<String>,
}
#[inline]
#[must_use]
pub fn resolve_package_registry_target(
    parsed: &Url,
    config: &DirectFetchConfig,
) -> Option<PackageRegistryTarget> {
    let host = parsed.host_str()?.to_ascii_lowercase();
    let parts = path_parts(parsed);
    if host == "pypi.org" {
        return pypi_name(&parts).map(|name| PackageRegistryTarget {
            request_url: format!("https://pypi.org/pypi/{}/json", urlencoding::encode(&name)),
            json_fields_first: Vec::new(),
            json_fields_last: vec!["releases".to_owned()],
        });
    }
    if contains(&config.npm_hosts, &host) {
        return npm_name(&host, &parts).map(|name| PackageRegistryTarget {
            request_url: format!("{}{}", config.npm_registry_url_prefix, npm_encode(&name)),
            json_fields_first: ["name", "description", "readme", "repository"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            json_fields_last: vec!["versions".to_owned()],
        });
    }
    if host == "crates.io" {
        return crates_name(&parts).map(|name| PackageRegistryTarget {
            request_url: format!(
                "https://crates.io/api/v1/crates/{}",
                urlencoding::encode(&name)
            ),
            json_fields_first: Vec::new(),
            json_fields_last: vec!["versions".to_owned()],
        });
    }
    None
}
#[inline]
pub fn format_package_registry_json(
    payload: &sonic_rs::Value,
    fields_first: &[String],
    fields_last: &[String],
) -> crate::Result<String> {
    if let Some(object) = payload.as_object() {
        let ordered = OrderedRegistryObject {
            object,
            fields_first,
            fields_last,
        };
        return sonic_rs::to_string_pretty(&ordered).map_err(|error| {
            crate::error::AppError::internal(format!("failed to serialize registry JSON: {error}"))
        });
    }
    sonic_rs::to_string_pretty(payload).map_err(|error| {
        crate::error::AppError::internal(format!("failed to serialize registry JSON: {error}"))
    })
}
struct OrderedRegistryObject<'payload> {
    object: &'payload sonic_rs::Object,
    fields_first: &'payload [String],
    fields_last: &'payload [String],
}
impl Serialize for OrderedRegistryObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.object.len()))?;
        for field in self.fields_first {
            if let Some(value) = self.object.get(field) {
                map.serialize_entry(field, value)?;
            }
        }
        for (key, value) in self.object {
            if !self.fields_first.iter().any(|field| field == key)
                && !self.fields_last.iter().any(|field| field == key)
            {
                map.serialize_entry(key, value)?;
            }
        }
        for field in self.fields_last {
            if !self.fields_first.iter().any(|first| first == field)
                && let Some(value) = self.object.get(field)
            {
                map.serialize_entry(field, value)?;
            }
        }
        map.end()
    }
}
fn pypi_name(parts: &[String]) -> Option<String> {
    let (section, rest) = parts.split_first()?;
    if section == "project" {
        let name = rest.first()?;
        return unquoted_segment(name);
    }
    if section == "pypi" && rest.len() == 2 && rest.get(1).is_some_and(|suffix| suffix == "json") {
        let name = rest.first()?;
        return unquoted_segment(name);
    }
    None
}
fn npm_name(host: &str, parts: &[String]) -> Option<String> {
    let package_parts = if host.starts_with("registry.") {
        parts
    } else {
        let (section, package_parts) = parts.split_first()?;
        if section != "package" || package_parts.is_empty() {
            return None;
        }
        package_parts
    };
    let decoded: Vec<String> = package_parts.iter().map(|part| decode(part)).collect();
    let name_parts: Vec<String> = if decoded.len() == 1 {
        let single = decoded.first()?;
        single.split('/').map(str::to_owned).collect()
    } else {
        decoded
    };
    if name_parts.len() == 1 {
        let name = name_parts.first()?;
        return is_segment(name).then(|| name.clone());
    }
    if name_parts.len() == 2 {
        let mut parts_iter = name_parts.iter();
        let scope = parts_iter.next()?;
        let name = parts_iter.next()?;
        let scope_name = scope.strip_prefix('@')?;
        return (is_segment(scope_name) && is_segment(name)).then(|| name_parts.join("/"));
    }
    None
}
fn crates_name(parts: &[String]) -> Option<String> {
    let (section, rest) = parts.split_first()?;
    if section == "crates" {
        let name = rest.first()?;
        return unquoted_segment(name);
    }
    if parts.len() == 4 {
        let mut parts_iter = parts.iter();
        let api = parts_iter.next()?;
        let version = parts_iter.next()?;
        let crates = parts_iter.next()?;
        let name = parts_iter.next()?;
        if api == "api" && version == "v1" && crates == "crates" {
            return unquoted_segment(name);
        }
    }
    None
}
fn unquoted_segment(value: &str) -> Option<String> {
    let decoded = decode(value);
    is_segment(&decoded).then_some(decoded)
}
fn is_segment(value: &str) -> bool {
    !value.is_empty() && !value.contains('/')
}
fn decode(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().into_owned()
}
fn path_parts(parsed: &Url) -> Vec<String> {
    parsed
        .path_segments()
        .map(|segments| {
            segments
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
fn npm_encode(value: &str) -> String {
    urlencoding::encode(value).replace("%40", "@")
}
fn contains(values: &[String], value: &str) -> bool {
    values
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(value))
}
