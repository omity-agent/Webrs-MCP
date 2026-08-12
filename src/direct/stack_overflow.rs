use crate::{Result, config::DirectFetchConfig, error::AppError};
use serde::{Deserialize, Serialize};
use url::Url;
const API_FILTER: &str = "W-vZ8WEHVi3D2JhQe1m8l90EjOxo6eCsb6b_6yfX0_p";
const MAX_ANSWERS_PER_REQUEST: &str = "100";
#[derive(Deserialize)]
struct StackExchangeResponse {
    items: Option<Vec<StackExchangeQuestion>>,
    #[serde(default)]
    has_more: bool,
    error_id: Option<i64>,
    error_name: Option<String>,
    error_message: Option<String>,
}
#[derive(Deserialize)]
struct StackExchangeQuestion {
    title: Option<String>,
    body_markdown: Option<String>,
    #[serde(default)]
    answers: Vec<StackExchangeAnswer>,
}
#[derive(Deserialize)]
struct StackExchangeAnswer {
    body_markdown: Option<String>,
}
#[derive(Serialize)]
struct QuestionAndAnswers {
    question: Question,
    answers: Vec<String>,
}
#[derive(Serialize)]
struct Question {
    title: String,
    body: String,
}
#[must_use]
#[inline]
pub fn resolve_stack_overflow_api_url(parsed: &Url, config: &DirectFetchConfig) -> Option<String> {
    let host = parsed.host_str()?.to_ascii_lowercase();
    if !contains(&config.stack_overflow_hosts, &host) {
        return None;
    }
    let question_id = question_id(parsed)?;
    let mut api = Url::parse(
        &config
            .stack_overflow_api_url_template
            .replace(concat!("{", "question_id", "}"), &question_id.to_string()),
    )
    .ok()?;
    api.query_pairs_mut()
        .append_pair("order", "desc")
        .append_pair("sort", "votes")
        .append_pair("site", "stackoverflow")
        .append_pair("page", "1")
        .append_pair("pagesize", MAX_ANSWERS_PER_REQUEST)
        .append_pair("filter", API_FILTER);
    Some(api.to_string())
}
#[inline]
pub fn format_stack_overflow_question_json(body: &[u8]) -> Result<String> {
    let response: StackExchangeResponse = sonic_rs::from_slice(body)
        .map_err(|_error| AppError::client("Stack Exchange API returned malformed JSON."))?;
    if let Some(message) = api_error_message(&response) {
        return Err(AppError::client(message));
    }
    if response.has_more {
        return Err(AppError::client(
            "Stack Exchange API returned more than 100 answers; direct fetch cannot return a complete answer list.",
        ));
    }
    let item = single_question(response.items)?;
    let output = QuestionAndAnswers {
        question: Question {
            title: required_question_field(item.title, "title")?,
            body: required_question_field(item.body_markdown, "body_markdown")?,
        },
        answers: answer_bodies(&item.answers)?,
    };
    sonic_rs::to_string_pretty(&output).map_err(|error| {
        AppError::internal(format!("failed to serialize Stack Overflow JSON: {error}"))
    })
}
fn question_id(parsed: &Url) -> Option<u64> {
    let parts: Vec<&str> = parsed
        .path_segments()?
        .filter(|part| !part.is_empty())
        .collect();
    let first = parts.first()?;
    let second = parts.get(1)?;
    if matches!(*first, "questions" | "q") {
        return parse_id(second);
    }
    None
}
fn parse_id(value: &str) -> Option<u64> {
    let parsed = value.parse::<u64>().ok()?;
    (parsed > 0).then_some(parsed)
}
fn single_question(items: Option<Vec<StackExchangeQuestion>>) -> Result<StackExchangeQuestion> {
    let mut questions = items
        .ok_or_else(|| AppError::client("Stack Exchange API response is missing questions."))?;
    if questions.len() != 1 {
        return Err(AppError::client(
            "Stack Exchange API did not return exactly one question.",
        ));
    }
    questions
        .pop()
        .ok_or_else(|| AppError::client("Stack Exchange API did not return exactly one question."))
}
fn required_question_field(value: Option<String>, key: &str) -> Result<String> {
    value
        .map(|content| content.replace("\r\n", "\n"))
        .ok_or_else(|| {
            AppError::client(format!("Stack Exchange API question is missing \"{key}\"."))
        })
}
fn answer_bodies(answers: &[StackExchangeAnswer]) -> Result<Vec<String>> {
    answers
        .iter()
        .map(|answer| {
            answer
                .body_markdown
                .clone()
                .map(|content| content.replace("\r\n", "\n"))
                .ok_or_else(|| AppError::client("Stack Exchange API returned an invalid answer."))
        })
        .collect()
}
fn api_error_message(response: &StackExchangeResponse) -> Option<String> {
    let error_id = response.error_id?;
    let error_name = response.error_name.as_deref().unwrap_or("unknown_error");
    let error_message = response
        .error_message
        .as_deref()
        .unwrap_or("Stack Exchange API rejected the question request.");
    Some(format!(
        "Stack Exchange API rejected the question request ({error_name}/{error_id}): {error_message}"
    ))
}
fn contains(values: &[String], value: &str) -> bool {
    values
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(value))
}
