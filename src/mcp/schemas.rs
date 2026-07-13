use crate::{
    Result,
    error::AppError,
    models::{
        FindArguments, FindResponse, OpenArguments, OpenResponse, SearchQueryArguments,
        SearchQueryResponse,
    },
};
use rmcp::{
    handler::server::tool::schema_for_output,
    model::{JsonObject, Tool},
};
use schemars::{JsonSchema, schema_for};
#[inline]
pub fn tools() -> Result<Vec<Tool>> {
    Ok(vec![
        tool::<SearchQueryArguments, SearchQueryResponse>(
            "search_query",
            "返回标题、日期、URL 与高亮内容。",
        )?,
        tool::<OpenArguments, OpenResponse>("open", "Open the page indicated by `url`.")?,
        tool::<FindArguments, FindResponse>(
            "find",
            "Find the text `pattern` in the page indicated by `url`.",
        )?,
    ])
}
#[inline]
pub fn tool_by_name(name: &str) -> Result<Option<Tool>> {
    Ok(tools()?.into_iter().find(|tool| tool.name == name))
}
fn tool<I, O>(name: &'static str, description: &'static str) -> Result<Tool>
where
    I: JsonSchema,
    O: JsonSchema + 'static,
{
    let output_schema = schema_for_output::<O>().map_err(|error| {
        AppError::internal(format!("failed to build tool output schema: {error}"))
    })?;
    Ok(Tool::new(name, description, schema_object::<I>()?).with_raw_output_schema(output_schema))
}
fn schema_object<T>() -> Result<JsonObject>
where
    T: JsonSchema,
{
    let value = rmcp::serde_json::to_value(schema_for!(T))
        .map_err(|error| AppError::internal(format!("failed to build tool schema: {error}")))?;
    match value {
        rmcp::serde_json::Value::Object(object) => Ok(object),
        rmcp::serde_json::Value::Null
        | rmcp::serde_json::Value::Bool(_)
        | rmcp::serde_json::Value::Number(_)
        | rmcp::serde_json::Value::String(_)
        | rmcp::serde_json::Value::Array(_) => {
            Err(AppError::internal("tool schema is not a JSON object"))
        }
    }
}
