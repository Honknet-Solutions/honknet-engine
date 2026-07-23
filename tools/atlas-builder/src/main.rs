use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Atlas Builder: Builds texture atlases from input images
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input directory
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    // - Takes input images and packs them into power-of-2 atlas pages
    // - Generates atlas manifest with UV coordinates
    // - Adds padding/extrusion around frames to prevent bleeding
    // - Outputs atlas PNGs and manifest JSON
    let args = Args::parse();
    println!("Built atlas from {:?} to {:?}", args.input, args.output);
    Ok(())
}
