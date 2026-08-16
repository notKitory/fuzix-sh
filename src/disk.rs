use crate::config::FuzixConfig;
use crate::toolchain::ToolchainManager;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;

pub struct DiskManager<'a> {
    pub config: &'a FuzixConfig,
    pub toolchain: &'a ToolchainManager,
}

impl<'a> DiskManager<'a> {
    pub fn new(config: &'a FuzixConfig, toolchain: &'a ToolchainManager) -> Self {
        Self { config, toolchain }
    }

    pub fn boot_image_path(&self) -> PathBuf {
        PathBuf::from(&self.config.disk.boot_image)
    }

    pub fn root_image_path(&self) -> PathBuf {
        PathBuf::from(&self.config.disk.root_image)
    }

    /// Ensure default disk images exist in the project, copying from runtime if needed.
    pub fn ensure_images(&self) -> Result<()> {
        let boot = self.boot_image_path();
        let root = self.root_image_path();

        if let Some(parent) = boot.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = root.parent() {
            fs::create_dir_all(parent)?;
        }

        let runtime = self.toolchain.runtime_dir();
        let emu = &self.config.target.emulator;
        let cpu = &self.config.target.cpu;

        if emu == "68knano" {
            let nano_disk = runtime.join("images").join("68knano-disk.img");
            let nano_rom = self.toolchain.emulators_dir().join("68knano.rom");

            if nano_disk.exists() {
                if !root.exists() {
                    fs::copy(&nano_disk, &root)?;
                }
            } else if !root.exists() {
                println!(
                    "{} Downloading official FUZIX 0.4 (68knano) disk image...",
                    "==>".cyan().bold()
                );
                if let Ok(resp) = ureq::get("https://fuzix.org/downloads/0.4/68knano/emu-ide.img").call() {
                    if let Ok(mut outfile) = fs::File::create(&root) {
                        let mut reader = resp.into_reader();
                        let _ = std::io::copy(&mut reader, &mut outfile);
                    }
                }
            }

            if !nano_rom.exists() {
                println!(
                    "{} Downloading official FUZIX 0.4 (68knano) ROM...",
                    "==>".cyan().bold()
                );
                if let Some(p) = nano_rom.parent() {
                    let _ = fs::create_dir_all(p);
                }
                if let Ok(resp) = ureq::get("https://fuzix.org/downloads/0.4/68knano/fuzix.rom").call() {
                    if let Ok(mut outfile) = fs::File::create(&nano_rom) {
                        let mut reader = resp.into_reader();
                        let _ = std::io::copy(&mut reader, &mut outfile);
                    }
                }
            }
        } else if emu == "v68" || emu == "tiny68k" || cpu == "68000" {
            let v68_img = runtime.join("images").join("v68-disk.img");
            if v68_img.exists() {
                if !root.exists() {
                    fs::copy(&v68_img, &root)?;
                }
            } else if !root.exists() {
                println!(
                    "{} Downloading official FUZIX 68000 disk image...",
                    "==>".cyan().bold()
                );
                if let Ok(resp) = ureq::get("https://fuzix.org/downloads/v68.tar.gz").call() {
                    let tar = GzDecoder::new(resp.into_reader());
                    let mut archive = Archive::new(tar);
                    if let Ok(entries) = archive.entries() {
                        for mut entry in entries.flatten() {
                            if let Ok(path) = entry.path() {
                                let p_str = path.to_string_lossy();
                                if p_str.ends_with("disk.img") || p_str.ends_with("drive.ide") || p_str.ends_with("fuzix.dsk") {
                                    if let Ok(mut outfile) = fs::File::create(&root) {
                                        let _ = std::io::copy(&mut entry, &mut outfile);
                                    }
                                } else if p_str.ends_with("boot.dat") {
                                    let boot_dat = self.toolchain.emulators_dir().join("boot.dat");
                                    if let Some(p) = boot_dat.parent() {
                                        let _ = fs::create_dir_all(p);
                                    }
                                    if let Ok(mut outfile) = fs::File::create(&boot_dat) {
                                        let _ = std::io::copy(&mut entry, &mut outfile);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let src_boot = runtime.join("images").join("boot.dsk");
            let src_hd = runtime.join("images").join("hd-fuzix.dsk");

            if !boot.exists() && src_boot.exists() {
                fs::copy(&src_boot, &boot)?;
            }
            if !root.exists() && src_hd.exists() {
                fs::copy(&src_hd, &root)?;
            }
        }

        Ok(())
    }

    /// Copy a host file onto the FUZIX disk image at the target path.
    pub fn copy_file<P: AsRef<Path>>(&self, src: P, dest_fuzix_path: &str, mode: Option<&str>) -> Result<()> {
        self.ensure_images()?;
        let src_path = src.as_ref();
        if !src_path.exists() {
            bail!("Host file not found: {}", src_path.display());
        }

        let ucp = self.toolchain.ucp_binary();
        if !ucp.exists() {
            bail!("ucp disk tool not found: {}", ucp.display());
        }

        let root_img = self.root_image_path();
        let src_file_name = src_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid source filename")?;

        let (dest_dir, dest_file_name) = if dest_fuzix_path.ends_with('/') {
            (dest_fuzix_path.trim_end_matches('/'), src_file_name)
        } else {
            let p = Path::new(dest_fuzix_path);
            let dir = p.parent().and_then(|p| p.to_str()).unwrap_or("/");
            let name = p.file_name().and_then(|p| p.to_str()).unwrap_or(src_file_name);
            (dir, name)
        };

        let dest_dir = if dest_dir.is_empty() { "/" } else { dest_dir };

        println!(
            "{} Copying {} -> {} on FUZIX disk",
            "==>".cyan().bold(),
            src_path.display().to_string().bold(),
            format!("{}/{}", dest_dir, dest_file_name).bold()
        );

        let parent_dir = src_path.parent().unwrap_or_else(|| Path::new("."));

        let mut ucp_script = format!("cd {}\nbget {}\n", dest_dir, src_file_name);
        if src_file_name != dest_file_name {
            ucp_script.push_str(&format!("mv {} {}\n", src_file_name, dest_file_name));
        }
        if let Some(m) = mode {
            ucp_script.push_str(&format!("chmod {} {}\n", m, dest_file_name));
        }
        ucp_script.push_str("exit\n");

        let mut child = Command::new(&ucp)
            .arg(&root_img)
            .current_dir(parent_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to run ucp: {}", ucp.display()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(ucp_script.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ucp command failed: {}", stderr);
        }

        println!("{} Injected file into disk successfully", "✓".green().bold());
        Ok(())
    }

    /// List directory contents inside the FUZIX disk image.
    pub fn list_dir(&self, fuzix_path: &str) -> Result<()> {
        self.ensure_images()?;
        let ucp = self.toolchain.ucp_binary();
        let root_img = self.root_image_path();

        let ucp_script = format!("cd {}\nls -l\nexit\n", fuzix_path);

        let mut child = Command::new(&ucp)
            .arg(&root_img)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to run ucp: {}", ucp.display()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(ucp_script.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);

        Ok(())
    }
}
