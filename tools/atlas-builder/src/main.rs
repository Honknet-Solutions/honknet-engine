use anyhow::{Context, Result};
use clap::Parser;
use image::{GenericImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Atlas Builder: Builds texture atlases from input images with UV maps
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input directory containing PNG images
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for atlas PNG and manifest JSON
    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct SpriteRect {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct AtlasManifest {
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub sprites: Vec<SpriteRect>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.output).context("Failed to create output directory")?;

    let mut images = Vec::new();
    if args.input.exists() && args.input.is_dir() {
        for entry in fs::read_dir(&args.input)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("png") {
                if let Ok(img) = image::open(&path) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    images.push((name, img));
                }
            }
        }
    }

    if images.is_empty() {
        println!("No PNG images found in {:?}", args.input);
        return Ok(());
    }

    // Grid layout algorithm
    let count = images.len() as u32;
    let cols = (count as f32).sqrt().ceil() as u32;
    let max_w = images
        .iter()
        .map(|(_, img)| img.width())
        .max()
        .unwrap_or(32);
    let max_h = images
        .iter()
        .map(|(_, img)| img.height())
        .max()
        .unwrap_or(32);

    let atlas_w = (cols * max_w).next_power_of_two();
    let rows = count.div_ceil(cols);
    let atlas_h = (rows * max_h).next_power_of_two();

    let mut atlas = RgbaImage::new(atlas_w, atlas_h);
    let mut rects = Vec::new();

    for (idx, (name, img)) in images.into_iter().enumerate() {
        let idx = idx as u32;
        let col = idx % cols;
        let row = idx / cols;
        let x = col * max_w;
        let y = row * max_h;

        let _ = atlas.copy_from(&img, x, y);

        rects.push(SpriteRect {
            name,
            x,
            y,
            width: img.width(),
            height: img.height(),
            u_min: x as f32 / atlas_w as f32,
            v_min: y as f32 / atlas_h as f32,
            u_max: (x + img.width()) as f32 / atlas_w as f32,
            v_max: (y + img.height()) as f32 / atlas_h as f32,
        });
    }

    let atlas_png_path = args.output.join("atlas.png");
    atlas.save(&atlas_png_path)?;

    let manifest = AtlasManifest {
        atlas_width: atlas_w,
        atlas_height: atlas_h,
        sprites: rects,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(args.output.join("atlas_manifest.json"), json)?;

    println!(
        "Packed {} images into {}x{} atlas at {:?}",
        manifest.sprites.len(),
        atlas_w,
        atlas_h,
        args.output
    );
    Ok(())
}
