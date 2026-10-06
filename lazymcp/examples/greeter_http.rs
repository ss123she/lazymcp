//! Greeter tool demonstrating optional parameters (`Option<T>`) and system instructions.
//!
//! Run with: `cargo run --example greeter_http --features="http" -- 127.0.0.1:8080`
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
    let mut args = std::env::args().skip(1);

    let addr = args.next().unwrap_or_else(|| "127.0.0.1:3000".to_string());

    let server = LazyMcp::new("greeter", "0.1.0")
        .with_instructions("Assistant that generates greetings.")
        .with_tool(greet_tool)
        .serve_http(addr)
        .await?;

    Ok(())
}
