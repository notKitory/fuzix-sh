use crate::config::FuzixConfig;
use crate::disk::DiskManager;
use crate::emulator::EmulatorRunner;
use crate::toolchain::ToolchainManager;
use anyhow::Result;

pub fn execute(
    command: String,
    args: Vec<String>,
    verbose: bool,
    emulator_override: Option<String>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<()> {
    toolchain.ensure_installed()?;
    let disk = DiskManager::new(config, toolchain);
    let runner = EmulatorRunner::new(config, toolchain, &disk);

    let full_command = if args.is_empty() {
        command
    } else {
        format!("{} {}", command, args.join(" "))
    };

    let output = runner.run_command(&full_command, verbose, None, emulator_override.as_deref())?;

    if !verbose && !output.trim().is_empty() {
        println!("{}", output);
    }

    Ok(())
}
