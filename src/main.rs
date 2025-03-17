use anyhow::{anyhow, Result};
use clap::Parser;
use comrak::{markdown_to_html, ComrakOptions};
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
        return Err(anyhow!("Failed to fetch documentation. Status: {}", response.status()));
    }
    
    let html_content = response.text()?;
    process_html_content(&html_content)
}

/// Fetch documentation by building locally
fn fetch_local_docs(crate_name: &str, item_path: Option<&str>) -> Result<String> {
    // Create a temporary directory for the operation
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();
    
    // Try to find the crate in the local cargo registry first
    let cargo_home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".cargo");
    
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
            return Err(anyhow!("Failed to build documentation for crate: {}", crate_name));
        }
        
        doc_path = temp_path.join("temp_project").join("target").join("doc");
    }
    
    // Find the HTML files
    let crate_doc_path = doc_path.join(crate_name.replace('-', "_"));
    
    if !crate_doc_path.exists() {
        return Err(anyhow!("Documentation not found for crate: {}", crate_name));
    }
    
    let index_path = if let Some(path) = item_path {
        crate_doc_path.join(path.replace("::", "/")).join("index.html")
    } else {
        crate_doc_path.join("index.html")
    };
    
    if !index_path.exists() {
        return Err(anyhow!("Documentation not found at path: {:?}", index_path));
    }
    
    let html_content = fs::read_to_string(index_path)?;
    process_html_content(&html_content)
}

/// Process HTML content to extract and convert relevant documentation parts
fn process_html_content(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    
    // Select the main content div which contains the documentation
    let main_content_selector = Selector::parse("#main-content").unwrap();
    let main_content = document
        .select(&main_content_selector)
        .next()
        .ok_or_else(|| anyhow!("Could not find main content section"))?;
    
    // Convert HTML to Markdown
    let html_content = main_content.inner_html();
    
    // First remove script tags which can cause issues
    let script_selector = Selector::parse("script").unwrap();
    let mut content_html = html_content.clone();
    for script in document.select(&script_selector) {
        let script_html = script.html();
        content_html = content_html.replace(&script_html, "");
    }
    
    // Simple HTML to plain text conversion
    let plain_text = html_to_text(&content_html)?;
    
    Ok(plain_text)
}

/// Convert HTML to plain text
fn html_to_text(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    
    // First extract text from HTML (simplified approach)
    let mut plain_text = String::new();
    extract_text_from_node(&document.root_element(), &mut plain_text);
    
    // Clean up the text (replace multiple newlines, etc.)
    let cleaned_text = plain_text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    
    Ok(cleaned_text)
}

/// Helper function to extract text from HTML nodes
fn extract_text_from_node(node: &scraper::Node, output: &mut String) {
    match node.value() {
        scraper::node::Node::Text(text) => {
            output.push_str(text);
            output.push('\n');
        }
        scraper::node::Node::Element(element) => {
            // Skip if it's a script or style tag
            if element.name.local.as_ref() == "script" || element.name.local.as_ref() == "style" {
                return;
            }
            
            // Add spacing for block elements
            let block_elements = ["div", "p", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "blockquote", "li"];
            if block_elements.contains(&element.name.local.as_ref()) {
                output.push('\n');
            }
            
            // Process child nodes
            for child in node.children() {
                extract_text_from_node(&child, output);
            }
            
            // Add spacing after block elements
            if block_elements.contains(&element.name.local.as_ref()) {
                output.push('\n');
            }
        }
        _ => {}
    }
}