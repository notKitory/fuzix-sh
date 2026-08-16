use crate::config::FuzixConfig;
use crate::disk::DiskManager;
use crate::emulator::EmulatorRunner;
use crate::toolchain::ToolchainManager;
use anyhow::Result;

pub fn execute(
    emulator_override: Option<String>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<()> {
    toolchain.ensure_installed()?;
    let disk = DiskManager::new(config, toolchain);
    let runner = EmulatorRunner::new(config, toolchain, &disk);

    runner.shell(emulator_override.as_deref())
}
