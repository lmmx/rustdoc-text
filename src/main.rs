#![doc = include_str!("../README.md")]
use anyhow::{anyhow, Result};
use clap::Parser;
use htmd::HtmlToMarkdown;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// A tool to view Rust documentation as plain text in the terminal
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The Rust crate name to fetch documentation for
    #[arg(index = 1)]
    crate_name: String,

    /// The item path within the crate (optional)
    #[arg(index = 2)]
    item_path: Option<String>,

    /// View the documentation from docs.rs instead of local build
    #[arg(short, long)]
    online: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let doc_content = if args.online {
        fetch_online_docs(&args.crate_name, args.item_path.as_deref())?
    } else {
        fetch_local_docs(&args.crate_name, args.item_path.as_deref())?
    };

    println!("{}", doc_content);

    Ok(())
}

/// Fetch documentation from docs.rs
fn fetch_online_docs(crate_name: &str, item_path: Option<&str>) -> Result<String> {
    let client = Client::new();

    // Construct the URL for docs.rs
    let mut url = format!("https://docs.rs/{}", crate_name);
    if let Some(path) = item_path {
        url = format!("{}/{}", url, path.replace("::", "/"));
    }

    // Fetch the HTML content
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch documentation. Status: {}",
            response.status()
        ));
    }

    let html_content = response.text()?;
    process_html_content(&html_content)
}

/// Fetch documentation by building locally
fn fetch_local_docs(crate_name: &str, item_path: Option<&str>) -> Result<String> {
    // Create a temporary directory for the operation
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Check if we're in a cargo project
    let current_dir = std::env::current_dir()?;
    let is_cargo_project = current_dir.join("Cargo.toml").exists();

    let doc_path: PathBuf;

    if is_cargo_project {
        // We're in a cargo project, build docs for the current project
        let status = Command::new("cargo")
            .args(["doc", "--no-deps"])
            .current_dir(&current_dir)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to build documentation with cargo doc"));
        }

        doc_path = current_dir.join("target").join("doc");
    } else {
        // Try to build documentation for an external crate
        let status = Command::new("cargo")
            .args(["new", "--bin", "temp_project"])
            .current_dir(temp_path)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to create temporary cargo project"));
        }

        // Add the crate as a dependency
        let temp_cargo_toml = temp_path.join("temp_project").join("Cargo.toml");
        let mut cargo_toml_content = fs::read_to_string(&temp_cargo_toml)?;
        cargo_toml_content.push_str(&format!("\n[dependencies]\n{} = \"*\"\n", crate_name));
        fs::write(&temp_cargo_toml, cargo_toml_content)?;

        // Build the documentation
        let status = Command::new("cargo")
            .args(["doc", "--no-deps"])
            .current_dir(temp_path.join("temp_project"))
            .status()?;

        if !status.success() {
            return Err(anyhow!(
                "Failed to build documentation for crate: {}",
                crate_name
            ));
        }

        doc_path = temp_path.join("temp_project").join("target").join("doc");
    }

    // Find the HTML files
    let crate_doc_path = doc_path.join(crate_name.replace('-', "_"));

    if !crate_doc_path.exists() {
        return Err(anyhow!("Documentation not found for crate: {}", crate_name));
    }

    let index_path = if let Some(path) = item_path {
        crate_doc_path
            .join(path.replace("::", "/"))
            .join("index.html")
    } else {
        crate_doc_path.join("index.html")
    };

    if !index_path.exists() {
        return Err(anyhow!("Documentation not found at path: {:?}", index_path));
    }

    let html_content = fs::read_to_string(index_path)?;
    process_html_content(&html_content)
}

/// Process HTML content to extract and convert relevant documentation parts to Markdown
fn process_html_content(html: &str) -> Result<String> {
    let document = Html::parse_document(html);

    // Select the main content div which contains the documentation
    let main_content_selector = Selector::parse("#main-content").unwrap();
    let main_content = document
        .select(&main_content_selector)
        .next()
        .ok_or_else(|| anyhow!("Could not find main content section"))?;

    // Get HTML content
    let html_content = main_content.inner_html();

    // Convert HTML to Markdown using htmd
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style"])
        .build();

    let markdown = converter
        .convert(&html_content)
        .map_err(|e| anyhow!("HTML to Markdown conversion failed: {}", e))?;

    // Clean up the markdown (replace multiple newlines, etc.)
    let cleaned_text = clean_markdown(&markdown);

    Ok(cleaned_text)
}

/// Clean up the markdown output to make it more readable in terminal
fn clean_markdown(markdown: &str) -> String {
    // Replace 3+ consecutive newlines with 2 newlines
    let mut result = String::new();
    let mut last_was_newline = false;
    let mut newline_count = 0;

    for c in markdown.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(c);
            }
            last_was_newline = true;
        } else {
            if last_was_newline {
                newline_count = 0;
                last_was_newline = false;
            }
            result.push(c);
        }
    }

    result
}
