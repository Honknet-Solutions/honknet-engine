use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Parser)]
#[command(name = "honk", version = "1.0.0-rc.1")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    New {
        template: String,
        name: String,
    },
    Dev,
    Run {
        target: String,
    },
    Studio,
    Test,
    Validate,
    Build,
    Package {
        #[arg(default_value = "dist")]
        output: PathBuf,
    },
    Publish,
    Profile,
    Replay {
        file: PathBuf,
    },
    Doctor,
    Clean,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::New { template, name } => new_project(&template, &name),
        Cmd::Dev => run("cargo", &["run", "-p", "honknet-server"]),
        Cmd::Run { target } => run("cargo", &["run", "-p", &format!("honknet-{target}")]),
        Cmd::Studio => run("npm", &["run", "studio"]),
        Cmd::Test => run("cargo", &["test", "--workspace", "--all-features"]),
        Cmd::Validate => validate(Path::new(".")),
        Cmd::Build => run("cargo", &["build", "--workspace", "--release"]),
        Cmd::Package { output } => package(&output),
        Cmd::Publish => {
            run("cargo", &["publish", "--dry-run"])?;
            println!("Dry-run passed; registry credentials are required for publication.");
            Ok(())
        }
        Cmd::Profile => run("cargo", &["run", "-p", "honknet-server", "--release"]),
        Cmd::Replay { file } => {
            let reader = honknet_replay::ReplayReader::open(&file)?;
            println!(
                "engine={} protocol={} content={} seed={}",
                reader.header.engine_version,
                reader.header.protocol,
                reader.header.content_hash,
                reader.header.seed
            );
            Ok(())
        }
        Cmd::Doctor => doctor(),
        Cmd::Clean => run("cargo", &["clean"]),
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {cmd}"))?;
    if !status.success() {
        bail!("{cmd} failed with {status}")
    }
    Ok(())
}

fn new_project(template: &str, name: &str) -> Result<()> {
    let source = Path::new("templates").join(template);
    if !source.is_dir() {
        bail!("unknown template {template}")
    }
    copy_dir(&source, Path::new(name))?;
    println!("Created {name} from {template}");
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        let to = dst.join(rel);
        if e.file_type().is_dir() {
            fs::create_dir_all(&to)?
        } else {
            fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

fn validate(root: &Path) -> Result<()> {
    let forbidden = [
        concat!("todo!", "()"),
        concat!("unimplemented!", "()"),
        concat!("panic!", "(\"not implemented\")"),
    ];
    let mut count = 0;
    for e in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let p = e.path();
        if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("rs" | "toml" | "yml" | "yaml" | "ts")
        ) {
            let s = fs::read_to_string(p)?;
            for f in forbidden {
                if s.contains(f) {
                    bail!("forbidden placeholder {f} in {}", p.display())
                }
            }
            count += 1
        }
    }
    println!("Validated {count} source/configuration files");
    Ok(())
}

fn package(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut manifest = String::new();
    for e in walkdir::WalkDir::new(".")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && !e.path().starts_with("./target")
                && !e.path().starts_with(out)
        })
    {
        let b = fs::read(e.path())?;
        manifest.push_str(&format!(
            "{}  {}
",
            hex::encode(Sha256::digest(&b)),
            e.path().display()
        ));
    }
    fs::write(out.join("SHA256SUMS"), manifest)?;
    println!("Package manifest written to {}", out.display());
    Ok(())
}

fn doctor() -> Result<()> {
    for (c, a) in [
        ("rustc", vec!["--version"]),
        ("cargo", vec!["--version"]),
        ("node", vec!["--version"]),
        ("npm", vec!["--version"]),
    ] {
        let ok = Command::new(c).args(a).status().is_ok_and(|s| s.success());
        println!("{} {c}", if ok { "OK " } else { "ERR" });
    }
    validate(Path::new("."))
}
