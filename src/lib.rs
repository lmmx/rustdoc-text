//! # rustdoc-text
//!
//! A lightweight library to view Rust documentation as plain text (Markdown).
//!
//! This crate provides both a library and a binary for accessing Rust documentation
//! in plain text format.
//!
#![doc = include_str!("../README.md")]

use anyhow::{anyhow, Result};
use htmd::HtmlToMarkdown;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Fetches Rust documentation from docs.rs and converts it to Markdown.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to fetch documentation for
/// * `item_path` - Optional path to a specific item within the crate
///
/// # Returns
///
/// The documentation as Markdown text.
///
/// # Examples
///
/// ```no_run
/// use rustdoc_text::fetch_online_docs;
///
/// # fn main() -> anyhow::Result<()> {
/// let docs = fetch_online_docs("serde", None)?;
/// println!("{}", docs);
/// # Ok(())
/// # }
/// ```
pub fn fetch_online_docs(crate_name: &str, item_path: Option<&str>) -> Result<String> {
    let client = Client::new();

    let url = if let Some(path) = item_path {
        // Parse the path to construct the proper docs.rs URL
        // Expected input format: "struct.Rope" or "module::struct.Name"
        let path_with_html = if !path.ends_with(".html") {
            format!("{}.html", path)
        } else {
            path.to_string()
        };

        // Replace :: with / for nested items
        let url_path = path_with_html.replace("::", "/");

        format!(
            "https://docs.rs/{}/latest/{}/{}",
            crate_name, crate_name, url_path
        )
    } else {
        format!("https://docs.rs/{}/latest/{}/", crate_name, crate_name)
    };

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

/// Builds and fetches Rust documentation locally and converts it to Markdown.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to fetch documentation for
/// * `item_path` - Optional path to a specific item within the crate
///
/// # Returns
///
/// The documentation as Markdown text.
///
/// # Examples
///
/// ```no_run
/// use rustdoc_text::fetch_local_docs;
///
/// # fn main() -> anyhow::Result<()> {
/// let docs = fetch_local_docs("serde", None)?;
/// println!("{}", docs);
/// # Ok(())
/// # }
/// ```
pub fn fetch_local_docs(crate_name: &str, item_path: Option<&str>) -> Result<String> {
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

/// Process HTML content to extract and convert relevant documentation parts to Markdown.
///
/// # Arguments
///
/// * `html` - The HTML content to process
///
/// # Returns
///
/// The documentation as Markdown text.
pub fn process_html_content(html: &str) -> Result<String> {
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

/// Clean up the markdown output to make it more readable in terminal.
///
/// # Arguments
///
/// * `markdown` - The markdown text to clean
///
/// # Returns
///
/// The cleaned markdown text.
pub fn clean_markdown(markdown: &str) -> String {
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

/// Configuration options for fetching Rust documentation.
pub struct Config {
    /// The name of the crate to fetch documentation for.
    pub crate_name: String,

    /// Optional path to a specific item within the crate.
    pub item_path: Option<String>,

    /// Whether to fetch documentation from docs.rs instead of building locally.
    pub online: bool,
}

impl Config {
    /// Create a new configuration with the specified crate name.
    ///
    /// # Arguments
    ///
    /// * `crate_name` - The name of the crate to fetch documentation for
    ///
    /// # Examples
    ///
    /// ```
    /// use rustdoc_text::Config;
    ///
    /// let config = Config::new("serde");
    /// assert_eq!(config.crate_name, "serde");
    /// assert_eq!(config.online, false);
    /// ```
    pub fn new<S: Into<String>>(crate_name: S) -> Self {
        Self {
            crate_name: crate_name.into(),
            item_path: None,
            online: false,
        }
    }

    /// Set the item path for the configuration.
    ///
    /// # Arguments
    ///
    /// * `item_path` - The item path within the crate
    ///
    /// # Examples
    ///
    /// ```
    /// use rustdoc_text::Config;
    ///
    /// let config = Config::new("serde").with_item_path("Deserializer");
    /// assert_eq!(config.item_path, Some("Deserializer".to_string()));
    /// ```
    pub fn with_item_path<S: Into<String>>(mut self, item_path: S) -> Self {
        self.item_path = Some(item_path.into());
        self
    }

    /// Set whether to fetch documentation from docs.rs.
    ///
    /// # Arguments
    ///
    /// * `online` - Whether to fetch documentation from docs.rs
    ///
    /// # Examples
    ///
    /// ```
    /// use rustdoc_text::Config;
    ///
    /// let config = Config::new("serde").with_online(true);
    /// assert_eq!(config.online, true);
    /// ```
    pub fn with_online(mut self, online: bool) -> Self {
        self.online = online;
        self
    }

    /// Execute the configuration to fetch documentation.
    ///
    /// # Returns
    ///
    /// The documentation as Markdown text.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rustdoc_text::Config;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let docs = Config::new("serde")
    ///     .with_online(true)
    ///     .with_item_path("Deserializer")
    ///     .execute()?;
    /// println!("{}", docs);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(&self) -> Result<String> {
        if self.online {
            fetch_online_docs(&self.crate_name, self.item_path.as_deref())
        } else {
            fetch_local_docs(&self.crate_name, self.item_path.as_deref())
        }
    }
}
