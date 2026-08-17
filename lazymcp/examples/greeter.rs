//! Greeter tool demonstrating optional parameters (`Option<T>`) and system instructions.
//!
//! Run with: `cargo run -p lazymcp --example greeter`
use lazymcp::{LazyMcp, tool};

/// Generate a customized greeting message.
#[tool]
fn greet(
    /// Name of the person to greet
    name: String,
    /// Optional honorific/title (e.g. "Dr.", "Captain")
    title: Option<String>,
    /// Whether to shout in UPPERCASE
    shout: Option<bool>,
) -> String {
    let full_name = match title {
        Some(t) => format!("{t} {name}"),
        None => name,
    };

    let msg = format!("Hello, {full_name}!");

    if shout.unwrap_or(false) {
        msg.to_uppercase()
    } else {
        msg
    }
}

#[lazymcp::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    LazyMcp::new("greeter", "0.1.0")
        .with_instructions("Assistant that generates greetings.")
        .with_tool(greet_tool)
        .serve_stdio()
        .await?;

    Ok(())
}
