use crate::config::FuzixConfig;
use crate::disk::DiskManager;
use crate::toolchain::ToolchainManager;
use anyhow::Result;
use std::path::PathBuf;

pub fn execute_cp(
    src: PathBuf,
    dest: String,
    mode: Option<String>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<()> {
    toolchain.ensure_installed()?;
    let disk = DiskManager::new(config, toolchain);
    disk.copy_file(src, &dest, mode.as_deref())
}

pub fn execute_ls(
    path: Option<String>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<()> {
    toolchain.ensure_installed()?;
    let disk = DiskManager::new(config, toolchain);
    let target_path = path.unwrap_or_else(|| "/bin".to_string());
    disk.list_dir(&target_path)
}
