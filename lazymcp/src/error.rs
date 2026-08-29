#[derive(Debug, Clone)]
pub enum McpError {
    InvalidArguments(String),
    ExecutionError(String),
    InternalError(String),
    MethodNotFound(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::InvalidArguments(msg) => write!(f, "invalid arguments: {msg}"),
            McpError::ExecutionError(msg) => write!(f, "execution error: {msg}"),
            McpError::InternalError(msg) => write!(f, "internal error: {msg}"),
            McpError::MethodNotFound(msg) => write!(f, "method not found: {msg}"),
        }
    }
}

impl std::error::Error for McpError {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn display_prefixes_message_with_variant_kind() {
        assert_eq!(
            McpError::InvalidArguments("bad input".into()).to_string(),
            "invalid arguments: bad input"
        );
        assert_eq!(
            McpError::ExecutionError("boom".into()).to_string(),
            "execution error: boom"
        );
        assert_eq!(
            McpError::InternalError("oops".into()).to_string(),
            "internal error: oops"
        );
        assert_eq!(
            McpError::MethodNotFound("no such tool".into()).to_string(),
            "method not found: no such tool"
        );
    }

    #[test]
    fn is_usable_as_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(McpError::ExecutionError("boom".into()));
        assert_eq!(err.to_string(), "execution error: boom");
    }

    #[test]
    fn maps_variants_to_jsonrpc_error_codes() {
        let cases = [
            (
                McpError::InvalidArguments(String::new()),
                ErrorCode::INVALID_PARAMS,
            ),
            (
                McpError::ExecutionError(String::new()),
                ErrorCode::INTERNAL_ERROR,
            ),
            (
                McpError::InternalError(String::new()),
                ErrorCode::INTERNAL_ERROR,
            ),
            (
                McpError::MethodNotFound(String::new()),
                ErrorCode::METHOD_NOT_FOUND,
            ),
        ];

        for (err, expected_code) in cases {
            assert_eq!(rmcp::model::ErrorData::from(err).code, expected_code);
        }
    }
}
