use super::oneshot::{Invocation, OutputFormat, SingleRequest};
use crate::{
    config::{ToolKind, ToolNames},
    models::{FindRequest, OpenRequest, SearchCategory, SearchQueryRequest},
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
    # [arg (long , value_parser = clap :: builder :: NonEmptyStringValueParser :: new ())]
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
    # [arg (long , value_parser = clap :: builder :: NonEmptyStringValueParser :: new ())]
    url: String,
    #[arg(long, default_value_t = 0)]
    chunk: usize,
    #[command(flatten)]
    presentation: Presentation,
}
#[derive(Args)]
struct FindInput {
    # [arg (long , value_parser = clap :: builder :: NonEmptyStringValueParser :: new ())]
    url: String,
    # [arg (long , allow_hyphen_values = true , value_parser = clap :: builder :: NonEmptyStringValueParser :: new ())]
    pattern: String,
    # [arg (long , value_parser = clap :: builder :: RangedU64ValueParser ::< usize >:: new () . range (1 ..))]
    snippet_tokens: Option<usize>,
    #[command(flatten)]
    presentation: Presentation,
}
pub(super) fn definitions(names: &ToolNames) -> [Command; 3] {
    [
        SearchInput::augment_args(
            Command::new(names.search_query.clone()).about("Search with one query"),
        ),
        OpenInput::augment_args(Command::new(names.open.clone()).about("Open one page chunk")),
        FindInput::augment_args(
            Command::new(names.find.clone()).about("Find a pattern in one page"),
        ),
    ]
}
pub(super) fn invocation(
    names: &ToolNames,
    matches: &ArgMatches,
) -> Result<Option<Invocation>, clap::Error> {
    let Some((name, arguments)) = matches.subcommand() else {
        return Ok(None);
    };
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
                    chunk: input.chunk,
                }),
                format: input.presentation.format,
            }
        }
        Some(ToolKind::Find) => {
            let input = FindInput::from_arg_matches(arguments)?;
            Invocation {
                request: SingleRequest::Find(FindRequest {
                    url: input.url,
                    pattern: input.pattern,
                    snippet_tokens: input.snippet_tokens,
                }),
                format: input.presentation.format,
            }
        }
        None => {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidSubcommand,
                format!("Unknown tool: {name}"),
            ));
        }
    };
    Ok(Some(invocation))
}
