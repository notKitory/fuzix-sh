use crate::config::FuzixConfig;
use crate::toolchain::ToolchainManager;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        let src_boot = runtime.join("images").join("boot.dsk");
        let src_hd = runtime.join("images").join("hd-fuzix.dsk");

        if !boot.exists() && src_boot.exists() {
            fs::copy(&src_boot, &boot)?;
        }
        if !root.exists() && src_hd.exists() {
            fs::copy(&src_hd, &root)?;
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
