use anyhow::Context;
use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

type Result<T> = anyhow::Result<T>;

#[derive(Deserialize)]
pub struct Config {
    pub package: Option<Package>,
    pub lib: Option<Target>,
    pub bin: Option<Vec<Target>>,
    pub test: Option<Vec<Target>>,
    pub bench: Option<Vec<Target>>,
    pub example: Option<Vec<Target>>,
}

impl Config {
    pub fn from_toml<P: AsRef<Path>>(file: P) -> Result<Self> {
        toml::from_str(str::from_utf8(&fs::read(file)?)?)
            .context("failed to read toml")
    }
}

#[derive(Deserialize)]
pub struct Package {
    pub build: Option<String>,
}

impl Package {
    pub fn build(&self) -> Option<Target> {
        self.build.as_ref().map(|s| Target { name: None, path: Some(s.clone()) })
    }
}

#[derive(Clone, Copy)]
pub enum TargetType {
    BuildScript,
    Library,
    Binary,
    Test,
    Bench,
    Example,
}

impl fmt::Display for TargetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TargetType::BuildScript => write!(f, "build-script"),
            TargetType::Library => write!(f, "library"),
            TargetType::Binary => write!(f, "binary"),
            TargetType::Test => write!(f, "test"),
            TargetType::Bench => write!(f, "bench"),
            TargetType::Example => write!(f, "example"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: Option<String>,
    pub path: Option<String>,
}

impl Target {
    pub fn path_and_content(&self, type_: TargetType) -> (PathBuf, String) {
        let path = if let Some(ref p) = self.path {
            PathBuf::from(&p)
        } else {
            match type_ {
                TargetType::BuildScript => {
                    PathBuf::from("build.rs")
                }
                TargetType::Library => {
                    PathBuf::from("src")
                        .join("lib.rs")
                }
                TargetType::Binary => {
                    PathBuf::from("src")
                        .join("main.rs")
                }
                TargetType::Test => {
                    PathBuf::from("tests")
                        .join(format!("{}.rs", self.name.as_ref().unwrap()))
                }
                TargetType::Bench => {
                    PathBuf::from("benches")
                        .join(format!("{}.rs", self.name.as_ref().unwrap()))
                }
                TargetType::Example => {
                    PathBuf::from("examples")
                        .join(format!("{}.rs", self.name.as_ref().unwrap()))
                }
            }
            
        };
        let content = match type_ {
            TargetType::Library
                | TargetType::Test
                | TargetType::Bench => "",
            TargetType::BuildScript
                | TargetType::Binary
                | TargetType::Example => "fn main() {}\n",
        };
        (path, content.to_string())
    }
}
