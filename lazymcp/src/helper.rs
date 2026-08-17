use rmcp::{
    Json,
    model::{CallToolResult, ContentBlock, TextContent},
};

pub trait IntoToolResult {
    fn into_tool_result(self) -> CallToolResult;
}

impl<T: IntoToolResult, E: std::fmt::Display> IntoToolResult for Result<T, E> {
    fn into_tool_result(self) -> CallToolResult {
        match self {
            Ok(val) => val.into_tool_result(),
            Err(err) => {
                let mut res = err.to_string().into_tool_result();
                res.is_error = Some(true);
                res
            }
        }
    }
}

impl<T: serde::Serialize> IntoToolResult for Json<T> {
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_string_pretty(&self.0) {
            Ok(text) => CallToolResult::success(vec![ContentBlock::Text(TextContent::new(text))]),
            Err(err) => {
                CallToolResult::error(vec![ContentBlock::Text(TextContent::new(err.to_string()))])
            }
        }
    }
}

impl IntoToolResult for String {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::Text(TextContent::new(self))])
    }
}

impl IntoToolResult for () {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::success(vec![])
    }
}

impl<T: IntoToolResult> IntoToolResult for Option<T> {
    fn into_tool_result(self) -> CallToolResult {
        match self {
            Some(val) => val.into_tool_result(),
            None => CallToolResult::success(vec![]),
        }
    }
}

impl IntoToolResult for CallToolResult {
    fn into_tool_result(self) -> CallToolResult {
        self
    }
}

macro_rules! impl_into_tool_result {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoToolResult for $ty {
                fn into_tool_result(self) -> CallToolResult {
                    self.to_string().into_tool_result()
                }
            }
        )*
    };
}

impl_into_tool_result!(
    bool, char, &str, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
);
