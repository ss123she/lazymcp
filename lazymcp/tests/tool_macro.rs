//! End-to-end tests for the `#[tool]` macro, driven through the public
//! `LazyMcp` API exactly the way user code consumes it.

use std::sync::atomic::{AtomicI32, Ordering};

use lazymcp::serde_json::{from_str, json};
use lazymcp::tokio;
use lazymcp::{CallToolResult, ContentBlock, Json, LazyMcp, McpError, State, tool};

/// Adds two integers.
#[tool]
fn add(
    /// First addend
    a: i32,
    /// Second addend
    b: i32,
) -> i32 {
    a + b
}

/// Generates a friendly greeting.
#[tool]
async fn greet(
    /// Who to greet
    name: String,
    /// Optional punctuation appended to the greeting
    punct: Option<String>,
) -> String {
    format!("Hello, {name}{}", punct.unwrap_or_default())
}

/// Divides `a` by `b`, reporting an error instead of panicking on zero.
#[tool]
fn safe_div(
    /// Dividend
    a: i32,
    /// Divisor
    b: i32,
) -> Result<i32, String> {
    a.checked_div(b)
        .ok_or_else(|| format!("division by zero: {a}/0"))
}

/// Returns the languages this server knows about.
#[tool]
fn supported_languages() -> Json<Vec<&'static str>> {
    Json(vec!["rust", "mcp"])
}

#[derive(Default)]
struct HitCounter {
    hits: AtomicI32,
}

/// Increments the shared hit counter and returns the new value.
#[tool]
async fn record_hit(state: State<HitCounter>) -> i32 {
    state.hits.fetch_add(1, Ordering::SeqCst) + 1
}

fn first_text(result: &CallToolResult) -> String {
    match result.content.first() {
        Some(ContentBlock::Text(text)) => text.text.clone(),
        other => panic!("expected text content, got {other:?}"),
    }
}

#[tokio::test]
async fn calls_sync_tools_with_typed_arguments() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(add_tool);

    let result = server
        .call_tool("add", json!({ "a": 2, "b": 3 }))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    assert_eq!(first_text(&result), "5");
}

#[tokio::test]
async fn calls_async_tools_and_defaults_missing_optional_arguments() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(greet_tool);

    let result = server
        .call_tool("greet", json!({ "name": "Ada" }))
        .await
        .unwrap();
    assert_eq!(first_text(&result), "Hello, Ada");

    let result = server
        .call_tool("greet", json!({ "name": "Ada", "punct": "!" }))
        .await
        .unwrap();
    assert_eq!(first_text(&result), "Hello, Ada!");
}

#[tokio::test]
async fn tool_errors_are_returned_as_error_results() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(safe_div_tool);

    let failure = server
        .call_tool("safe_div", json!({ "a": 1, "b": 0 }))
        .await
        .unwrap();

    assert_eq!(failure.is_error, Some(true));
    assert!(first_text(&failure).contains("division by zero"));

    let success = server
        .call_tool("safe_div", json!({ "a": 6, "b": 3 }))
        .await
        .unwrap();

    assert_eq!(success.is_error, Some(false));
    assert_eq!(first_text(&success), "2");
}

#[tokio::test]
async fn malformed_arguments_return_invalid_arguments_errors() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(add_tool);

    let wrong_type = server
        .call_tool("add", json!({ "a": "two", "b": 3 }))
        .await
        .unwrap_err();
    assert!(matches!(wrong_type, McpError::InvalidArguments(_)));

    let missing_field = server.call_tool("add", json!({})).await.unwrap_err();
    assert!(matches!(missing_field, McpError::InvalidArguments(_)));
}

#[tokio::test]
async fn json_return_values_round_trip_through_the_payload() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(supported_languages_tool);

    let result = server
        .call_tool("supported_languages", json!({}))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));

    let languages: Vec<String> = from_str(&first_text(&result)).unwrap();
    assert_eq!(languages, ["rust", "mcp"]);
}

#[tokio::test]
async fn state_is_injected_into_tools() {
    let server = LazyMcp::new("test", "0.0.0")
        .with_state(HitCounter::default())
        .with_tool(record_hit_tool);

    let first = server.call_tool("record_hit", json!({})).await.unwrap();
    let second = server.call_tool("record_hit", json!({})).await.unwrap();

    assert_eq!(first_text(&first), "1");
    assert_eq!(first_text(&second), "2");
}

#[tokio::test]
async fn missing_state_fails_with_an_internal_error() {
    let server = LazyMcp::new("test", "0.0.0").with_tool(record_hit_tool);

    let err = server.call_tool("record_hit", json!({})).await.unwrap_err();

    match err {
        McpError::InternalError(msg) => {
            assert!(msg.contains("HitCounter"), "unexpected message: {msg}");
            assert!(msg.contains("not registered"), "unexpected message: {msg}");
        }
        other => panic!("expected InternalError, got: {other:?}"),
    }
}

#[test]
fn tool_listings_expose_docs_and_schemas() {
    let server = LazyMcp::new("test", "0.0.0")
        .with_tool(add_tool)
        .with_tool(greet_tool);

    let tools = server.list_tools();

    let add = tools
        .iter()
        .find(|t| t.name == "add")
        .expect("add tool listed");
    assert_eq!(add.description.as_deref(), Some("Adds two integers."));

    let schema = &add.input_schema;
    assert_eq!(
        schema["properties"]["a"]["description"].as_str(),
        Some("First addend")
    );

    let required = schema["required"].as_array().expect("required list");
    assert!(required.iter().any(|value| value == "a"));
    assert!(required.iter().any(|value| value == "b"));

    let greet = tools
        .iter()
        .find(|t| t.name == "greet")
        .expect("greet tool listed");
    let greet_required = greet.input_schema["required"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        !greet_required.iter().any(|value| value == "punct"),
        "optional argument must not be required: {greet_required:?}"
    );
}
