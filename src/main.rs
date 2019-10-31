// TODO: add toml, read tomls and make sure we handle [[bin]] and [[test]] and [[bench]]

use anyhow::anyhow;
use std::ffi;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

mod config;

type Result<T> = anyhow::Result<T>;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(long, parse(from_os_str))]
    output_dir: ffi::OsString,
}

fn is_target_dir(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s == "target")
        .unwrap_or(false)
}

fn is_cargo_toml(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s == "Cargo.toml")
        .unwrap_or(false)
}

fn is_main_or_lib_or_build(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s == "main.rs" || s == "lib.rs" || s == "build.rs")
        .unwrap_or(false)
}

// When invoked as a cargo subcommand, cargo passes too many arguments so we need to filter out
// arg[1] if it matches the end of arg[0], e.i. "cargo-X X foo" should become "cargo-X foo".
fn args() -> impl Iterator<Item = String> {
    let mut args: Vec<String> = ::std::env::args().collect();

    if args.len() >= 2 {
        if args[0].ends_with(&format!("cargo-{}", args[1])) {
            args.remove(1);
        }
    }

    args.into_iter()
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let dst_dir = dst.parent().unwrap();
    fs::create_dir_all(dst_dir)?;
    fs::copy(src, dst)?;
    Ok(())
}

fn create_file(dst: &Path, content: &str) -> Result<()> {
    let dst_dir = dst.parent().unwrap();
    fs::create_dir_all(dst_dir)?;
    fs::write(dst, content)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::from_iter(args());
    
    if !Path::new("Cargo.toml").exists() {
        return Err(anyhow!("no Cargo.toml found"));
    }

    let walker: Vec<_> = WalkDir::new(".")
        .into_iter()
        .filter_entry(|e| !is_target_dir(e))
        .filter_map(|e| e.ok())
        .filter(|e| is_cargo_toml(e) || is_main_or_lib_or_build(e))
        .collect();


    let lockfile = Path::new("./Cargo.lock");
    if lockfile.exists() {
        let dst = Path::new(&args.output_dir).join(lockfile.strip_prefix(".")?);
        println!("copying {} to {}", lockfile.display(), dst.display());
        copy_file(lockfile, &dst)?;
    }

    let toolchain = Path::new("./rust-toolchain");
    if toolchain.exists() {
        let dst = Path::new(&args.output_dir).join(toolchain.strip_prefix(".")?);
        println!("copying {} to {}", toolchain.display(), dst.display());
        copy_file(toolchain, &dst)?;
    }
    
    for entry in walker {
        let src = entry.path();
        let dst = PathBuf::from(&args.output_dir).join(entry.path().strip_prefix(".")?);
        if is_cargo_toml(&entry) {
            println!("copying {} to {}", src.display(), dst.display());
            copy_file(src, &dst)?;

            let conf = config::Config::from_toml(src)?;

            let mut target_shadows = vec![];
            let build: Option<config::Target> = conf.package.and_then(|p| p.build());
            target_shadows.extend(
                build
                    .iter()
                    .map(|t| (t, config::TargetType::BuildScript))
            );
            target_shadows.extend(
                conf
                    .lib
                    .iter()
                    .map(|t| (t, config::TargetType::Library))
            );
            target_shadows.extend(
                conf
                    .bin
                    .iter()
                    .flatten()
                    .map(|t| (t, config::TargetType::Binary))
            );
            target_shadows.extend(
                conf
                    .test
                    .iter()
                    .flatten()
                    .map(|t| (t, config::TargetType::Test))
            );
            target_shadows.extend(
                conf
                    .bench
                    .iter()
                    .flatten()
                    .map(|t| (t, config::TargetType::Bench))
            );
            target_shadows.extend(
                conf
                    .example
                    .iter()
                    .flatten()
                    .map(|t| (t, config::TargetType::Example))
            );
            for (target, type_) in &target_shadows {
                let (path, content) = target.path_and_content(*type_);
                let target_dst = dst.parent().unwrap().join(&path);
                println!("shadowing {} to {}", type_, target_dst.display());
                create_file(&target_dst, &content)?;
            }
        } else {
            println!("shadowing {} to {}", src.display(), dst.display());
            let default_path = src.file_name().unwrap();
            if default_path == "main.rs" || default_path == "build.rs" {
                create_file(&dst, "fn main() { }\n")?;
            }
            if default_path == "lib.rs" {
                create_file(&dst, "\n")?;
            }
        }
    }
    
    Ok(())
}
