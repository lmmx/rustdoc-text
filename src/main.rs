use anyhow::Result;
use clap::Parser;
use rustdoc_text::{fetch_local_docs, fetch_online_docs};

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
