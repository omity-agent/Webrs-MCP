use super::ToolOutput;
use crate::{
    Result,
    mcp::server::tool_result,
    models::{
        FindPage, FindResponse, OpenPage, OpenResponse, OpenResult, SearchQueryResponse,
        SearchResult,
    },
};
use sonic_rs::JsonValueTrait as _;
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps serialization failures readable."
)]
fn search_output_is_raw_unindented_pseudo_xml_and_json() -> Result<()> {
    let output = ToolOutput::Search(SearchQueryResponse {
        results: vec![SearchResult {
            title: Some("A&B <C>".to_owned()),
            date: None,
            url: "https://example.com/?a=1&b=2".to_owned(),
            highlight: "first line\n<SECOND>raw</SECOND>".to_owned(),
        }],
        warning: Some(vec!["x < y & z".to_owned()]),
    });
    assert_eq!(
        output.standard_text(),
        concat!(
            "<RESULTS>\n",
            "<RESULT>\n",
            "<TITLE>\nA&B <C>\n</TITLE>\n",
            "<URL>\nhttps://example.com/?a=1&b=2\n</URL>\n",
            "<HIGHLIGHT>\nfirst line\n<SECOND>raw</SECOND>\n</HIGHLIGHT>\n",
            "</RESULT>\n",
            "</RESULTS>\n",
            "<WARNINGS>\n",
            "<WARNING>\nx < y & z\n</WARNING>\n",
            "</WARNINGS>"
        )
    );
    let structured = output.structured()?;
    assert_eq!(structured["results"][0]["title"], "A&B <C>");
    assert!(structured["results"][0]["date"].is_null());
    assert_eq!(
        structured["results"][0]["highlight"],
        "first line\n<SECOND>raw</SECOND>"
    );
    assert_eq!(structured["warning"][0], "x < y & z");
    Ok(())
}
#[test]
fn open_output_preserves_success_and_failure_shapes() {
    let output = ToolOutput::Open(OpenResponse {
        results: vec![
            OpenResult::Success {
                url: "https://example.com/ok".to_owned(),
                page: OpenPage {
                    chunk: 1,
                    total_chunks: 2,
                    content: "raw <body> & text".to_owned(),
                },
            },
            OpenResult::Failure {
                url: "https://example.com/error".to_owned(),
                error: "fetch failed".to_owned(),
            },
        ],
        warning: None,
    });
    assert_eq!(
        output.standard_text(),
        concat!(
            "<RESULTS>\n",
            "<RESULT>\n",
            "<URL>\nhttps://example.com/ok\n</URL>\n",
            "<PAGE>\n",
            "<CHUNK>\n1\n</CHUNK>\n",
            "<TOTAL_CHUNKS>\n2\n</TOTAL_CHUNKS>\n",
            "<CONTENT>\nraw <body> & text\n</CONTENT>\n",
            "</PAGE>\n",
            "</RESULT>\n",
            "<RESULT>\n",
            "<URL>\nhttps://example.com/error\n</URL>\n",
            "<ERROR>\nfetch failed\n</ERROR>\n",
            "</RESULT>\n",
            "</RESULTS>"
        )
    );
}
#[test]
fn find_output_keeps_empty_containers_and_populates_both_mcp_views() {
    let output = ToolOutput::Find(FindResponse {
        pages: vec![FindPage {
            total_chunks: 3,
            matches: Vec::new(),
        }],
        warning: Some(Vec::new()),
    });
    assert_eq!(
        output.standard_text(),
        concat!(
            "<PAGES>\n",
            "<PAGE>\n",
            "<TOTAL_CHUNKS>\n3\n</TOTAL_CHUNKS>\n",
            "<MATCHES>\n\n</MATCHES>\n",
            "</PAGE>\n",
            "</PAGES>\n",
            "<WARNINGS>\n\n</WARNINGS>"
        )
    );
    let result = tool_result(&output).unwrap_or_else(|error| panic!("{error}"));
    let encoded = rmcp::serde_json::to_value(result)
        .unwrap_or_else(|error| panic!("failed to encode CallToolResult: {error}"));
    assert_eq!(
        encoded
            .pointer("/content/0/text")
            .and_then(rmcp::serde_json::Value::as_str),
        Some(output.standard_text().as_str())
    );
    assert_eq!(
        encoded
            .pointer("/structuredContent/pages/0/total_chunks")
            .and_then(rmcp::serde_json::Value::as_u64),
        Some(3_u64)
    );
    assert_eq!(
        encoded
            .get("isError")
            .and_then(rmcp::serde_json::Value::as_bool),
        Some(false)
    );
}
