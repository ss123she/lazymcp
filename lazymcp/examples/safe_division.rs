use lazymcp::{LazyMcp, tool};

#[tool]
/// Function that safely divides a by b.
fn safe_div(
    /// dividend
    a: i32,
    /// divisor
    b: i32,
) -> Result<i32, &'static str> {
    a.checked_div(b).ok_or("ERROR: Division by zero.")
}

#[lazymcp::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    LazyMcp::new("safe-divison", "0.1.0")
        .with_tool(safe_div_tool)
        .serve_stdio()
        .await?;
    Ok(())
}
