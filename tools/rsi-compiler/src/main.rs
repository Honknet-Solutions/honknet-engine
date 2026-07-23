use anyhow::{Context, Result};
use clap::Parser;
use honknet_rsi::RsiReader;
use image::{GenericImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// RSI Compiler: Compiles RSI sources into texture atlases and runtime manifests
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input directory of .rsi folders
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory
    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompiledRsiManifest {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub states_count: usize,
    pub atlas_file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.output).context("Failed to create output directory")?;

    let mut compiled_count = 0;
    if args.input.exists() && args.input.is_dir() {
        for entry in fs::read_dir(&args.input)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.extension().and_then(|s| s.to_str()) == Some("rsi") {
                let mut reader = RsiReader::new(&path);
                if let Ok(meta) = reader.read_meta() {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    // Generate a composite atlas page for this RSI
                    let mut atlas = RgbaImage::new(
                        meta.size[0].max(64) * 4,
                        meta.size[1].max(64) * meta.states.len().max(1) as u32,
                    );
                    let mut y_offset = 0;

                    for state in &meta.states {
                        let img_path = path.join(format!("{}.png", state.name));
                        if img_path.exists() {
                            if let Ok(src_img) = image::open(&img_path) {
                                let _ = atlas.copy_from(&src_img, 0, y_offset);
                            }
                        }
                        y_offset += meta.size[1];
                    }

                    let atlas_file_name = format!("{name}_atlas.png");
                    let atlas_path = args.output.join(&atlas_file_name);
                    atlas.save(&atlas_path)?;

                    let manifest = CompiledRsiManifest {
                        name: name.clone(),
                        width: atlas.width(),
                        height: atlas.height(),
                        states_count: meta.states.len(),
                        atlas_file: atlas_file_name,
                    };

                    let manifest_json = serde_json::to_string_pretty(&manifest)?;
                    fs::write(
                        args.output.join(format!("{name}_manifest.json")),
                        manifest_json,
                    )?;

                    compiled_count += 1;
                }
            }
        }
    }

    println!(
        "Successfully compiled {compiled_count} RSI assets to {:?}",
        args.output
    );
    Ok(())
}
