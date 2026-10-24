use crate::{
    Result,
    cli::WorkMode,
    config::ToolNames,
    error::AppError,
    models::{
        ChunkSelection, FilesystemOpenArguments, FilesystemResponse, FilesystemSearchArguments,
        FindArguments, FindResponse, OpenArguments, OpenResponse, PagePartition,
        SearchQueryArguments, SearchQueryResponse, SplitCount, Unpartitioned,
    },
};
use alloc::sync::Arc;
use rmcp::model::{JsonObject, Tool};
use schemars::{JsonSchema, generate::SchemaSettings, schema_for, transform::RestrictFormats};
#[inline]
pub fn tools(names: &ToolNames, mode: WorkMode, chunking: bool) -> Result<Vec<Tool>> {
    if mode == WorkMode::Filesystem {
        return Ok(vec![
            tool::<FilesystemSearchArguments, FilesystemResponse>(
                &names.search_query,
                "Search for pages.",
            )?,
            tool::<FilesystemOpenArguments, FilesystemResponse>(&names.open, "Fetch the pages.")?,
        ]);
    }
    let search = tool::<SearchQueryArguments, SearchQueryResponse>(
        &names.search_query,
        "返回标题、日期、URL 与高亮内容。",
    )?;
    let (open, find) = if chunking {
        (
            tool::<OpenArguments<ChunkSelection>, OpenResponse<PagePartition>>(
                &names.open,
                "Open a chunk of the page indicated by `url`.",
            )?,
            tool::<FindArguments, FindResponse<SplitCount, ChunkSelection>>(
                &names.find,
                "Find the text `pattern` in chunks of the page indicated by `url`.",
            )?,
        )
    } else {
        (
            tool::<OpenArguments<Unpartitioned>, OpenResponse<Unpartitioned>>(
                &names.open,
                "Open the full page indicated by `url`.",
            )?,
            tool::<FindArguments, FindResponse<Unpartitioned, Unpartitioned>>(
                &names.find,
                "Find the text `pattern` in the full page indicated by `url`.",
            )?,
        )
    };
    Ok(vec![search, open, find])
}
fn tool<I, O>(name: &str, description: &'static str) -> Result<Tool>
where
    I: JsonSchema,
    O: JsonSchema,
{
    let output_schema = output_schema::<O>()?;
    Ok(
        Tool::new(name.to_owned(), description, schema_object::<I>()?)
            .with_raw_output_schema(output_schema),
    )
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
