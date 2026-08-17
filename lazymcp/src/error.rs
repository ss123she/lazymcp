#[derive(Debug, Clone)]
pub enum McpError {
    InvalidArguments(String),
    ExecutionError(String),
    InternalError(String),
    MethodNotFound(String),
}

impl From<McpError> for rmcp::model::ErrorData {
    fn from(err: McpError) -> Self {
        match err {
            McpError::InvalidArguments(msg) => rmcp::model::ErrorData {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: msg.into(),
                data: None,
            },
            McpError::ExecutionError(msg) | McpError::InternalError(msg) => {
                rmcp::model::ErrorData {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: msg.into(),
                    data: None,
                }
            }
            McpError::MethodNotFound(msg) => rmcp::model::ErrorData {
                code: rmcp::model::ErrorCode::METHOD_NOT_FOUND,
                message: msg.into(),
                data: None,
            },
        }
    }
}
