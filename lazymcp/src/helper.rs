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

#[cfg(test)]
mod tests {
    use super::*;

    fn first_text(result: &CallToolResult) -> String {
        match result.content.first() {
            Some(ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn string_becomes_successful_text() {
        let result = String::from("hello").into_tool_result();

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "hello");
    }

    #[test]
    fn primitives_render_as_text() {
        assert_eq!(first_text(&42i32.into_tool_result()), "42");
        assert_eq!(first_text(&true.into_tool_result()), "true");
    }

    #[test]
    fn unit_yields_an_empty_success() {
        let result = ().into_tool_result();

        assert_eq!(result.is_error, Some(false));
        assert!(result.content.is_empty());
    }

    #[test]
    fn option_none_yields_an_empty_success() {
        let result: Option<String> = None;
        let result = result.into_tool_result();

        assert_eq!(result.is_error, Some(false));
        assert!(result.content.is_empty());
    }

    #[test]
    fn option_some_delegates_to_inner() {
        let result = Some(String::from("hi")).into_tool_result();

        assert_eq!(first_text(&result), "hi");
    }

    #[test]
    fn result_ok_delegates_to_inner() {
        let outcome: Result<String, &str> = Ok(String::from("fine"));
        let result = outcome.into_tool_result();

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "fine");
    }

    #[test]
    fn result_err_is_flagged_as_an_error_result() {
        let outcome: Result<String, &str> = Err("division by zero");
        let result = outcome.into_tool_result();

        assert_eq!(result.is_error, Some(true));
        assert_eq!(first_text(&result), "division by zero");
    }

    #[test]
    fn json_values_round_trip_through_the_text_payload() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Point {
            x: i32,
            y: i32,
        }

        let result = Json(Point { x: 1, y: -2 }).into_tool_result();

        assert_eq!(result.is_error, Some(false));
        let parsed: Point = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(parsed, Point { x: 1, y: -2 });
    }

    #[test]
    fn call_tool_result_passes_through_unchanged() {
        let original = CallToolResult::success(vec![]);
        let result = original.into_tool_result();

        assert_eq!(result.is_error, Some(false));
        assert!(result.content.is_empty());
    }
}
