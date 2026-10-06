//! Thin, ergonomic macros for building MCP servers directly on the
//! official [`rmcp`](https://docs.rs/rmcp) SDK.

pub mod error;
pub mod helper;
pub mod state;

pub use error::McpError;
pub use helper::IntoToolResult;
pub use lazymcp_macros::{main, tool};
pub use rmcp;
pub use rmcp::Json;
pub use rmcp::model::{CallToolResult, ContentBlock, TextContent};
pub use schemars;
pub use serde;
pub use serde_json;
pub use state::State;
pub use tokio;

use rmcp::model::{CallToolResponse, ResultType, ServerCapabilities, ServerConfig};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type StateMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

/// A single MCP tool. Implemented automatically by `#[tool]`.
pub trait McpTool {
    /// Tool name.
    fn name(&self) -> &'static str;

    /// Tool description (generated automatically from doc comments).
    fn description(&self) -> Option<&'static str>;

    /// JSON Schema.
    fn schema(&self) -> Arc<rmcp::model::JsonObject>;

    /// Runs the tool with the given arguments.
    fn call<'a>(
        &'a self,
        arguments: serde_json::Value,
        states: &'a StateMap,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + 'a>>;
}

/// Builder for an MCP server.
pub struct LazyMcp {
    name: String,
    version: String,
    tools: HashMap<String, Box<dyn McpTool + Send + Sync>>,
    states: StateMap,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
}

impl LazyMcp {
    /// Creates a new server.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: HashMap::new(),
            states: HashMap::new(),
            instructions: None,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
        }
    }

    /// Registers shared state, wrapped in an `Arc`.
    pub fn with_state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.states.insert(TypeId::of::<T>(), Arc::new(state));
        self
    }

    /// Registers shared state you already hold as an `Arc`.
    pub fn with_arc_state<T: Send + Sync + 'static>(mut self, state: Arc<T>) -> Self {
        self.states.insert(TypeId::of::<T>(), state);
        self
    }

    /// Gets registered state by type, if any.
    pub fn get_state<T: Send + Sync + 'static>(&self) -> Option<State<T>> {
        self.states
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|arc| arc.downcast::<T>().ok())
            .map(State)
    }

    /// Sets the server's instructions for clients.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Overrides the server's capabilities.
    pub fn with_capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Registers a tool.
    ///
    /// # Panics
    ///
    /// Panics if a tool with the same name is already registered.
    pub fn with_tool<T>(mut self, tool: T) -> Self
    where
        T: McpTool + Send + Sync + 'static,
    {
        let name = tool.name().to_string();

        if self.tools.contains_key(&name) {
            panic!("lazymcp: tool '{name}' is already registered");
        }

        self.tools.insert(name, Box::new(tool));
        self
    }

    /// Lists registered tools with their schemas.
    pub fn list_tools(&self) -> Vec<rmcp::model::Tool> {
        self.tools
            .iter()
            .map(|(name, tool)| {
                let schema = tool.schema();
                let mut t = rmcp::model::Tool::new(name.clone(), "", schema);

                t.description = tool.description().map(std::borrow::Cow::from);
                t
            })
            .collect()
    }

    /// Calls a registered tool by name.
    ///
    /// # Errors
    /// Returns `McpError::MethodNotFound` if no tool has this name.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        if let Some(tool) = self.tools.get(name) {
            tool.call(arguments, &self.states).await
        } else {
            Err(McpError::MethodNotFound(format!("Tool '{name}' not found")))
        }
    }

    /// Serves the server over stdio.
    pub async fn serve_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        use rmcp::ServiceExt;

        let running_service = self
            .serve((tokio::io::stdin(), tokio::io::stdout()))
            .await?;

        running_service.waiting().await?;

        Ok(())
    }
}

impl rmcp::ServerHandler for LazyMcp {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::new(self.capabilities.clone()).with_server_info(
            rmcp::model::Implementation::new(self.name.clone(), self.version.clone()),
        );

        if let Some(instructions) = &self.instructions {
            info = info.with_instructions(instructions.clone());
        }

        info
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let tools = self.list_tools();

        let result = rmcp::model::ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
            result_type: Some(ResultType::COMPLETE),
            ttl_ms: None,
            cache_scope: None,
        };

        Ok(result)
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        self.call_tool(&request.name, args)
            .await
            .map(CallToolResponse::from)
            .map_err(rmcp::model::ErrorData::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct EchoTool;

    impl McpTool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> Option<&'static str> {
            Some("Echoes the text argument")
        }

        fn schema(&self) -> Arc<rmcp::model::JsonObject> {
            Arc::new(serde_json::Map::new())
        }

        fn call<'a>(
            &'a self,
            arguments: serde_json::Value,
            _states: &'a StateMap,
        ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + 'a>> {
            Box::pin(async move {
                let text = arguments
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                Ok(text.into_tool_result())
            })
        }
    }

    fn first_text(result: &CallToolResult) -> String {
        match result.content.first() {
            Some(rmcp::model::ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn calls_a_registered_tool() {
        let server = LazyMcp::new("test-server", "0.0.0").with_tool(EchoTool);

        let result = server
            .call_tool("echo", json!({ "text": "hi" }))
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "hi");
    }

    #[tokio::test]
    async fn calling_an_unknown_tool_returns_method_not_found() {
        let server = LazyMcp::new("test-server", "0.0.0");

        let err = server.call_tool("missing", json!({})).await.unwrap_err();

        assert!(matches!(err, McpError::MethodNotFound(_)));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    #[should_panic(expected = "already registered")]
    fn registering_a_duplicate_tool_name_panics() {
        LazyMcp::new("test-server", "0.0.0")
            .with_tool(EchoTool)
            .with_tool(EchoTool);
    }

    #[test]
    fn state_is_retrievable_by_type() {
        let server = LazyMcp::new("test-server", "0.0.0").with_state(42u64);

        assert_eq!(*server.get_state::<u64>().expect("u64 state"), 42);
        assert!(server.get_state::<String>().is_none());
    }

    #[test]
    fn registering_the_same_state_type_replaces_it() {
        let server = LazyMcp::new("test-server", "0.0.0")
            .with_state(1u8)
            .with_state(2u8);

        assert_eq!(*server.get_state::<u8>().expect("u8 state"), 2);
    }

    #[test]
    fn arc_state_is_shared_not_copied() {
        let arc = Arc::new(String::from("shared"));
        let server = LazyMcp::new("test-server", "0.0.0").with_arc_state(Arc::clone(&arc));

        let state = server.get_state::<String>().expect("String state");

        assert!(Arc::ptr_eq(&arc, &state.0));
    }

    #[test]
    fn list_tools_exposes_names_and_descriptions() {
        let tools = LazyMcp::new("test-server", "0.0.0")
            .with_tool(EchoTool)
            .list_tools();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(
            tools[0].description.as_deref(),
            Some("Echoes the text argument")
        );
    }
}
