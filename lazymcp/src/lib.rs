//! Thin, ergonomic macros for building MCP servers directly on the
//! official [`rmcp`](https://docs.rs/rmcp) SDK.
//!
//! Unlike independent reimplementations of the MCP protocol, lazymcp
//! wraps the reference Rust SDK - so you inherit spec compliance and
//! transport support instead of maintaining a parallel implementation.

pub mod error;
pub mod helper;
pub mod state;

pub use error::McpError;
pub use helper::IntoToolResult;
pub use lazymcp_macros::tool;
pub use rmcp;
pub use rmcp::Json;
pub use rmcp::model::{CallToolResult, ContentBlock, TextContent};
pub use schemars;
pub use serde;
pub use serde_json;
pub use state::State;
pub use tokio;

use rmcp::model::{CallToolResponse, ResultType, ServerCapabilities, ServerInfo};
use rmcp::service::MaybeSendFuture;
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
    pub fn with_tool<T>(mut self, tool: T) -> Self
    where
        T: McpTool + Send + Sync + 'static,
    {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
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
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(self.capabilities.clone()).with_server_info(
            rmcp::model::Implementation::new(self.name.clone(), self.version.clone()),
        );

        if let Some(instructions) = &self.instructions {
            info = info.with_instructions(instructions.clone());
        }

        info
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListToolsResult, rmcp::ErrorData>> + MaybeSendFuture + '_
    {
        async move {
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
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, rmcp::ErrorData>> + MaybeSendFuture + '_
    {
        async move {
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
}
