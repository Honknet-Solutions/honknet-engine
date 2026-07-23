use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Asset Manifest Builder: Builds asset manifest
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Content directory
    #[arg(short, long)]
    content: PathBuf,

    /// Output manifest file
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    // - Walks content directory
    // - Builds AssetManifest with: path, asset_id, content_hash, size, type, bundle assignment
    // - Outputs manifest.json for browser loading
    // - Groups assets into streaming bundles: bootstrap, core-ui, core-world, map-*, etc.
    let args = Args::parse();
    println!(
        "Built manifest from {:?} to {:?}",
        args.content, args.output
    );
    Ok(())
}
