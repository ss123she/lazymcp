# LazyMCP

[![Crates.io](https://img.shields.io/crates/v/lazymcp.svg)](https://crates.io/crates/lazymcp)
[![Documentation](https://docs.rs/lazymcp/badge.svg)](https://docs.rs/lazymcp)
[![License](https://img.shields.io/crates/l/lazymcp.svg)](#license)

An ergonomic, boilerplate-free framework for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io) servers in Rust.

**Think of `rmcp` as `hyper`, and `lazymcp` as `axum`.**  
`lazymcp` is a high-level, ergonomic layer built on top of [`rmcp`](https://crates.io/crates/rmcp) designed for developer happiness and zero boilerplate.

## Features

- **`#[tool]` Macro**: Turn any sync or async function into an MCP tool.
- **Auto Schemas & Docs**: Doc comments on functions and arguments (`///`) are automatically extracted into tool descriptions and JSON schemas.
- **Dependency Injection**: Inject shared state using `State<T>`.
- **Flexible Returns**: Support for `String`, `Json<T>`, `Result<T, E>`, primitives, or custom types.

## Why lazymcp instead of an independent MCP implementation?

Most Rust MCP frameworks (`pmcp`, `rust-mcp-sdk`, `ultrafast-mcp`) reimplement
the protocol from scratch. `lazymcp` doesn't — it's a thin layer on the
official [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) SDK, so
you inherit spec compliance, transport support, and protocol fixes from the
reference implementation instead of a parallel one.

| | lazymcp | Independent SDKs (pmcp, ultrafast-mcp, ...) |
|---|---|---|
| Protocol implementation | `rmcp` (official) | own |
| Spec updates | inherited automatically | maintained independently |
| Tool arguments | plain function parameters | separate params struct |
| Argument docs | `///` on each parameter | doc comment on struct field |

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
lazymcp = "0.1"
```

```rust
use lazymcp::{LazyMcp, tool};

/// Safely divides `a` by `b`.
#[tool]
fn safe_div(
    /// The dividend
    a: i32,
    /// The divisor
    b: i32,
) -> Result<i32, &'static str> {
    a.checked_div(b).ok_or("ERROR: Division by zero.")
}

#[lazymcp::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    LazyMcp::new("safe-division", "0.1.0")
        .with_tool(safe_div_tool)
        .serve_stdio()
        .await?;

    Ok(())
}
```

**Note:** The `#[tool]` macro automatically generates a `<function_name>_tool` struct to pass to `.with_tool()`.

## Examples
Check out the [`lazymcp/examples/`](lazymcp/examples/) directory:
- [`safe_division.rs`](lazymcp/examples/safe_division.rs) — Error handling with `Result` and auto-generated docs.
- [`greeter.rs`](lazymcp/examples/greeter.rs) — Optional arguments (`Option<T>`) and system instructions.
- [`custom_types.rs`](lazymcp/examples/custom_types.rs) — Stateful CRUD task tracker with custom structs, enums, and `Json<T>`.

You can run any example directly with Cargo:
```bash
cargo run -p lazymcp --example custom_types
```

## License
Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
