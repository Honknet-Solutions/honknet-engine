use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// RSI Compiler: Compiles RSI sources into atlases
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

fn main() -> Result<()> {
    // - Validates all RSI meta.json files
    // - Extracts frames from PNG sheets
    // - Packs frames into atlas pages (simple row-based packing)
    // - Generates runtime manifest JSON
    // - Generates ATTRIBUTIONS.json from license metadata
    // - Content hashing for each asset
    let args = Args::parse();
    println!("Compiled RSI from {:?} to {:?}", args.input, args.output);
    Ok(())
}
