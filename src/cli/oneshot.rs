use crate::{
    Result,
    config::ToolKind,
    error::AppError,
    models::{OpenRequest, SearchQueryRequest},
};
use clap::ValueEnum;
use sonic_rs::Value;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[non_exhaustive]
pub enum OutputFormat {
    #[default]
    Xml,
    Json,
}
#[derive(Clone, Debug)]
pub struct Invocation {
    pub request: SingleRequest,
    pub format: OutputFormat,
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SingleRequest {
    Search(SearchQueryRequest),
    Open(OpenRequest),
}
impl SingleRequest {
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> ToolKind {
        match *self {
            Self::Search(_) => ToolKind::Search,
            Self::Open(_) => ToolKind::Open,
        }
    }
    pub(crate) fn into_arguments(self) -> Result<(ToolKind, Value)> {
        let kind = self.kind();
        let encoded = match self {
            Self::Search(request) => sonic_rs::to_value(&request),
            Self::Open(request) => sonic_rs::to_value(&request),
        }
        .map_err(|error| AppError::internal(format!("failed to encode CLI request: {error}")))?;
        Ok((kind, sonic_rs :: json ! ({ "requests" : [encoded] })))
    }
}
