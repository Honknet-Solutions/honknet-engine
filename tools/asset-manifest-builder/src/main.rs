use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Asset Manifest Builder: Scans game content, hashes files, and produces manifest.json
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

#[derive(Serialize, Deserialize, Debug)]
struct ManifestItem {
    pub logical_path: String,
    pub asset_id: u32,
    pub content_hash: String,
    pub size_bytes: u64,
    pub asset_type: String,
    pub bundle: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AssetManifestFile {
    pub version: u32,
    pub generated_at: String,
    pub total_assets: usize,
    pub assets: Vec<ManifestItem>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut items = Vec::new();
    let mut id_counter = 1000u32;

    if args.content.exists() {
        for entry in WalkDir::new(&args.content)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                let bytes = fs::read(path)?;

                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let hash = hex::encode(hasher.finalize());

                let rel_path = path
                    .strip_prefix(&args.content)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");

                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let bundle = match ext.as_str() {
                    "png" | "jpg" | "webp" => "core-world",
                    "json" | "toml" | "yml" => "bootstrap",
                    "ogg" | "wav" => "audio",
                    _ => "default",
                };

                items.push(ManifestItem {
                    logical_path: rel_path,
                    asset_id: id_counter,
                    content_hash: hash,
                    size_bytes: bytes.len() as u64,
                    asset_type: ext,
                    bundle: bundle.to_string(),
                });

                id_counter += 1;
            }
        }
    }

    let manifest = AssetManifestFile {
        version: 1,
        generated_at: "2026-07-23".to_string(),
        total_assets: items.len(),
        assets: items,
    };

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&args.output, json)
        .with_context(|| format!("Failed to write manifest to {:?}", args.output))?;

    println!(
        "Manifest successfully built with {} assets to {:?}",
        manifest.total_assets, args.output
    );

    Ok(())
}
