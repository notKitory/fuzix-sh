use anyhow::{bail, Context, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

pub struct ToolchainManager {
    pub base_dir: PathBuf,
    pub arch: String,
    pub repo: String,
    pub release: String,
}

impl ToolchainManager {
    pub fn new(repo: &str, release: &str) -> Result<Self> {
        let arch = detect_host_arch()?;
        let base_dir = dirs::home_dir()
            .context("Could not determine user home directory")?
            .join(".fuzix");

        Ok(Self {
            base_dir,
            arch,
            repo: repo.to_string(),
            release: release.to_string(),
        })
    }

    pub fn toolchain_dir(&self) -> PathBuf {
        self.base_dir.join("toolchain").join(&self.arch)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.base_dir.join("runtime")
    }

    pub fn emulators_dir(&self) -> PathBuf {
        self.base_dir.join("emulators").join(&self.arch)
    }

    pub fn fcc_binary(&self) -> PathBuf {
        self.toolchain_dir().join("bin").join("fcc")
    }

    pub fn ucp_binary(&self) -> PathBuf {
        self.runtime_dir().join("ucp")
    }

    pub fn emulator_binary(&self, name: &str) -> PathBuf {
        self.emulators_dir().join(name)
    }

    /// Check if necessary toolchain components are present; if not, download them.
    pub fn ensure_installed(&self) -> Result<()> {
        let marker = self.base_dir.join(format!(".complete_{}", self.arch));
        if marker.exists() && self.fcc_binary().exists() && self.ucp_binary().exists() {
            return Ok(());
        }

        fs::create_dir_all(&self.base_dir)
            .with_context(|| format!("Failed to create {}", self.base_dir.display()))?;

        println!(
            "{} Initializing native FUZIX toolchain and emulators ({})",
            "==>".bright_green().bold(),
            self.arch.bold()
        );

        let assets = [
            (
                format!("fuzix-toolchain-{}.tar.gz", self.arch),
                self.toolchain_dir(),
            ),
            (
                format!("fuzix-runtime-{}.tar.gz", self.arch),
                self.runtime_dir(),
            ),
            (
                format!("emulatorkit-runtime-{}.tar.gz", self.arch),
                self.emulators_dir(),
            ),
            (
                format!("z80pack-runtime-{}.tar.gz", self.arch),
                self.emulators_dir(),
            ),
        ];

        for (asset_name, dest_dir) in assets {
            self.download_and_extract(&asset_name, &dest_dir)?;
        }

        // Set executable permissions on unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let make_exec = |path: &Path| {
                if path.is_file() {
                    if let Ok(metadata) = path.metadata() {
                        let mut perms = metadata.permissions();
                        perms.set_mode(perms.mode() | 0o755);
                        let _ = fs::set_permissions(path, perms);
                    }
                }
            };

            for entry in walkdir(&self.toolchain_dir().join("bin")) {
                make_exec(&entry);
            }
            make_exec(&self.ucp_binary());
            for entry in walkdir(&self.emulators_dir()) {
                make_exec(&entry);
            }
        }

        // Ensure boot.dat is present for v68
        let boot_dat = self.emulators_dir().join("boot.dat");
        if !boot_dat.exists() {
            self.ensure_boot_dat(&boot_dat);
        }

        fs::write(&marker, self.release.as_bytes())
            .context("Failed to write toolchain completion marker")?;

        println!(
            "{} Toolchain and emulators installed successfully!",
            "✓".green().bold()
        );

        Ok(())
    }

    fn ensure_boot_dat(&self, dest: &Path) {
        // Try direct download of boot.dat
        let direct_url = format!(
            "https://github.com/{}/releases/download/{}/boot.dat",
            self.repo,
            if self.release == "latest" { "prebuilt-latest" } else { &self.release }
        );
        if let Ok(resp) = ureq::get(&direct_url).call() {
            let mut buf = Vec::new();
            if resp.into_reader().read_to_end(&mut buf).is_ok() && buf.len() >= 4096 {
                let _ = fs::write(dest, buf);
                return;
            }
        }

        // Try extracting boot.dat from linux-amd64 runtime package
        let linux_tar_url = format!(
            "https://github.com/{}/releases/download/{}/emulatorkit-runtime-linux-amd64.tar.gz",
            self.repo,
            if self.release == "latest" { "prebuilt-latest" } else { &self.release }
        );
        if let Ok(resp) = ureq::get(&linux_tar_url).call() {
            let mut buf = Vec::new();
            if resp.into_reader().read_to_end(&mut buf).is_ok() {
                let tar = GzDecoder::new(&buf[..]);
                let mut archive = Archive::new(tar);
                if let Ok(entries) = archive.entries() {
                    for mut entry in entries.flatten() {
                        if let Ok(path) = entry.path() {
                            if path.ends_with("boot.dat") {
                                let _ = entry.unpack(dest);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    fn download_and_extract(&self, asset_name: &str, dest_dir: &Path) -> Result<()> {
        fs::create_dir_all(dest_dir)
            .with_context(|| format!("Failed to create directory {}", dest_dir.display()))?;

        let url = if self.release == "latest" {
            format!(
                "https://github.com/{}/releases/latest/download/{}",
                self.repo, asset_name
            )
        } else {
            format!(
                "https://github.com/{}/releases/download/{}/{}",
                self.repo, self.release, asset_name
            )
        };

        println!("    {} Downloading {}", "•".cyan(), asset_name.bold());

        let response = match ureq::get(&url).call() {
            Ok(resp) => resp,
            Err(e) => {
                bail!(
                    "Failed to download {} from {}: {}\nCheck your internet connection or release settings.",
                    asset_name, url, e
                );
            }
        };

        let total_size = response
            .header("Content-Length")
            .and_then(|l| l.parse::<u64>().ok());

        let pb = match total_size {
            Some(size) => {
                let pb = ProgressBar::new(size);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("      [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"),
                );
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(ProgressStyle::default_spinner().template("      {spinner:.green} {bytes}").unwrap());
                pb
            }
        };

        let mut reader = pb.wrap_read(response.into_reader());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        pb.finish_and_clear();

        // Extract tar.gz in-memory
        let tar = GzDecoder::new(&buffer[..]);
        let mut archive = Archive::new(tar);
        archive
            .unpack(dest_dir)
            .with_context(|| format!("Failed to unpack {} into {}", asset_name, dest_dir.display()))?;

        Ok(())
    }
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}

pub fn detect_host_arch() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let target = match (os, arch) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("windows", "x86_64") => "windows-amd64",
        _ => bail!("Unsupported host OS and architecture: {}-{}", os, arch),
    };

    Ok(target.to_string())
}
