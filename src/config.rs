use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "fuzix.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzixConfig {
    pub project: ProjectConfig,
    pub target: TargetConfig,
    #[serde(default)]
    pub disk: DiskConfig,
    #[serde(default)]
    pub toolchain: ToolchainConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_source")]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    #[serde(default = "default_cpu")]
    pub cpu: String,
    #[serde(default = "default_emulator")]
    pub emulator: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    #[serde(default = "default_boot_image")]
    pub boot_image: String,
    #[serde(default = "default_root_image")]
    pub root_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    #[serde(default = "default_repo")]
    pub repo: String,
    #[serde(default = "default_release")]
    pub release: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_source() -> String {
    "hello.c".to_string()
}

fn default_cpu() -> String {
    "68000".to_string()
}

fn default_emulator() -> String {
    "v68".to_string()
}

fn default_timeout() -> u64 {
    45
}

fn default_boot_image() -> String {
    ".fuzix/images/boot.dsk".to_string()
}

fn default_root_image() -> String {
    ".fuzix/images/hd-fuzix.dsk".to_string()
}

fn default_repo() -> String {
    "notKitory/fuzix-sh".to_string()
}

fn default_release() -> String {
    "latest".to_string()
}

impl Default for DiskConfig {
    fn default() -> Self {
        Self {
            boot_image: default_boot_image(),
            root_image: default_root_image(),
        }
    }
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        Self {
            repo: default_repo(),
            release: default_release(),
        }
    }
}

impl Default for FuzixConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "fuzix-app".to_string(),
                version: default_version(),
                source: default_source(),
            },
            target: TargetConfig {
                cpu: default_cpu(),
                emulator: default_emulator(),
                timeout: default_timeout(),
            },
            disk: DiskConfig::default(),
            toolchain: ToolchainConfig::default(),
        }
    }
}

impl FuzixConfig {
    /// Search for `fuzix.toml` in the given or current directory.
    pub fn find(start_dir: Option<&Path>) -> Result<Option<(PathBuf, Self)>> {
        let mut cur = match start_dir {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir().context("Failed to get current directory")?,
        };

        loop {
            let candidate = cur.join(CONFIG_FILE_NAME);
            if candidate.is_file() {
                let content = fs::read_to_string(&candidate)
                    .with_context(|| format!("Failed to read {}", candidate.display()))?;
                let config: FuzixConfig = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse {}", candidate.display()))?;
                return Ok(Some((candidate, config)));
            }

            if !cur.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Load config from file or return default.
    pub fn load_or_default(start_dir: Option<&Path>) -> (Option<PathBuf>, Self) {
        match Self::find(start_dir) {
            Ok(Some((path, config))) => (Some(path), config),
            _ => (None, Self::default()),
        }
    }

    /// Save configuration to path.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration to TOML")?;
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write {}", path.as_ref().display()))?;
        Ok(())
    }
}
