use crate::{
    Result,
    error::AppError,
    models::{
        FindArguments, FindResponse, OpenArguments, OpenResponse, SearchQueryArguments,
        SearchQueryResponse,
    },
};
use alloc::sync::Arc;
use rmcp::model::{JsonObject, Tool};
use schemars::{JsonSchema, generate::SchemaSettings, schema_for, transform::RestrictFormats};
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
    O: JsonSchema,
{
    let output_schema = output_schema::<O>()?;
    Ok(Tool::new(name, description, schema_object::<I>()?).with_raw_output_schema(output_schema))
}
fn output_schema<T>() -> Result<Arc<JsonObject>>
where
    T: JsonSchema,
{
    let settings = SchemaSettings::draft2020_12()
        .for_serialize()
        .with(|settings| settings.inline_subschemas = true)
        .with_transform(RestrictFormats::default());
    let schema = settings.into_generator().into_root_schema_for::<T>();
    let value = rmcp::serde_json::to_value(schema).map_err(|error| {
        AppError::internal(format!("failed to build tool output schema: {error}"))
    })?;
    let rmcp::serde_json::Value::Object(mut object) = value else {
        return Err(AppError::internal(
            "tool output schema is not a JSON object",
        ));
    };
    if object.get("type").and_then(rmcp::serde_json::Value::as_str) != Some("object") {
        return Err(AppError::internal(
            "tool output schema root type is not object",
        ));
    }
    object.remove("title");
    object.remove("description");
    Ok(Arc::new(object))
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
