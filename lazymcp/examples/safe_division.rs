//! Math tool demonstrating `Result` error handling and auto-generated docs.
//!
//! Run with: `cargo run -p lazymcp --example safe_division`
use lazymcp::{LazyMcp, tool};

/// Function that safely divides a by b.
#[tool]
fn safe_div(
    /// dividend
    a: i32,
    /// divisor
    b: i32,
) -> Result<i32, &'static str> {
    a.checked_div(b).ok_or("ERROR: Division by zero.")
}

#[lazymcp::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    LazyMcp::new("safe-division", "0.1.0")
        .with_tool(safe_div_tool)
        .serve_stdio()
        .await?;
    Ok(())
}
