# rustdoc-text

A lightweight CLI tool to view Rust documentation as plain text (Markdown) in the terminal.

Similar to tools like `pydoc` and `godoc`, but for Rust documentation.

## Features

- View documentation for any Rust crate directly in your terminal as Markdown
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

This tool:

1. Fetches Rust documentation (either by building locally or from docs.rs)
2. Extracts the main content section from the HTML
3. Converts the HTML to Markdown using the htmd library
4. Outputs clean, readable Markdown to stdout

## Why Markdown?

Markdown is a lightweight markup language that's very readable as plain text, making it ideal for terminal output. It preserves the structure of the documentation while being much more readable than raw HTML.

## Dependencies

- `htmd`: For HTML to Markdown conversion
- `clap`: For command-line argument parsing
- `reqwest`: For fetching online documentation
- `anyhow`: For error handling
- `scraper`: For HTML parsing

## License

This project is licensed under the MIT License - see the LICENSE file for details.