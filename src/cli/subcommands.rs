use super::oneshot::{Invocation, OutputFormat, SingleRequest};
use crate::{
    config::{ToolKind, ToolNames},
    models::{OpenRequest, SearchCategory, SearchQueryRequest},
};
use clap::{ArgMatches, Args, Command, FromArgMatches as _};
#[derive(Args)]
struct Presentation {
    #[arg(
        long,
        value_enum,
        default_value_t,
        help = "Output format: raw pseudo XML or JSON"
    )]
    format: OutputFormat,
}
#[derive(Args)]
struct SearchInput {
    #[arg(value_name = "QUERY", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    q: String,
    # [arg (long , value_parser = clap :: value_parser ! (u64) . range (1 ..))]
    recency: Option<u64>,
    # [arg (long , value_delimiter = ',' , value_parser = clap :: builder :: NonEmptyStringValueParser :: new ())]
    domains: Vec<String>,
    #[arg(long, value_enum)]
    category: Option<SearchCategory>,
    #[command(flatten)]
    presentation: Presentation,
}
#[derive(Args)]
struct OpenInput {
    #[arg(value_name = "URL", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    url: String,
    #[command(flatten)]
    presentation: Presentation,
}
pub(super) fn definitions(names: &ToolNames) -> [Command; 2] {
    [
        SearchInput::augment_args(
            Command::new(names.search_query.clone()).about("Search with one query"),
        ),
        OpenInput::augment_args(Command::new(names.open.clone()).about("Open one full page")),
    ]
}
pub(super) fn invocation(
    names: &ToolNames,
    name: &str,
    arguments: &ArgMatches,
) -> Result<Invocation, clap::Error> {
    let invocation = match names.resolve(name) {
        Some(ToolKind::Search) => {
            let input = SearchInput::from_arg_matches(arguments)?;
            Invocation {
                request: SingleRequest::Search(SearchQueryRequest {
                    q: input.q,
                    recency: input.recency,
                    domains: (!input.domains.is_empty()).then_some(input.domains),
                    category: input.category,
                }),
                format: input.presentation.format,
            }
        }
        Some(ToolKind::Open) => {
            let input = OpenInput::from_arg_matches(arguments)?;
            Invocation {
                request: SingleRequest::Open(OpenRequest {
                    url: input.url,
                    selection: None,
                }),
                format: input.presentation.format,
            }
        }
        Some(ToolKind::Find) | None => {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidSubcommand,
                format!("Unknown tool: {name}"),
            ));
        }
    };
    Ok(invocation)
}
