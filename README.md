# rustdoc-text

A lightweight CLI tool to view Rust documentation as plain text in the terminal.

Similar to tools like `pydoc` and `godoc`, but for Rust documentation.

## Features

- View documentation for any Rust crate directly in your terminal
- Access documentation locally (builds as needed) or from docs.rs
- Lightweight and fast with minimal dependencies
- Simple command-line interface

## Installation

```bash
cargo install rustdoc-text
```

## Usage

```bash
# View documentation for a crate
rustdoc-text serde

# View documentation for a specific item in a crate
rustdoc-text serde Deserializer

# View documentation from docs.rs (instead of building locally)
rustdoc-text --online tokio

# Get help
rustdoc-text --help
```

## How it works

This tool works in two modes:

1. **Local mode** (default): Creates a temporary cargo project, adds the requested crate as a dependency, builds documentation with `cargo doc`, extracts the HTML content, and converts it to plain text.

2. **Online mode** (with `--online` flag): Fetches the documentation directly from docs.rs and converts it to plain text.

## Publishing

This crate is available on [crates.io](https://crates.io/crates/rustdoc-text).

## License

This project is licensed under the MIT License - see the LICENSE file for details.
