pub mod aliases;
pub(crate) mod field;
pub mod normalizer;
pub(crate) mod request;
pub(crate) mod singleton;
pub(crate) mod support;
pub mod typed;
use crate::{
    Result,
    models::{FindArguments, OpenArguments, SearchQueryArguments},
    warnings::Warnings,
};
use sonic_rs::Value;
#[derive(Clone, Debug)]
pub struct Normalized<T> {
    pub value: T,
    pub warnings: Warnings,
}
impl<T> Normalized<T> {
    #[inline]
    #[must_use]
    pub const fn new(value: T, warnings: Warnings) -> Self {
        Self { value, warnings }
    }
}
#[inline]
pub fn search_arguments(raw: Option<Value>) -> Result<Normalized<SearchQueryArguments>> {
    typed::search_arguments(raw)
}
#[inline]
pub fn open_arguments(raw: Option<Value>) -> Result<Normalized<OpenArguments>> {
    typed::open_arguments(raw)
}
#[inline]
pub fn find_arguments(raw: Option<Value>) -> Result<Normalized<FindArguments>> {
    typed::find_arguments(raw)
}
