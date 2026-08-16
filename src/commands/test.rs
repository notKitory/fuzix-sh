use crate::commands::build;
use crate::commands::disk;
use crate::commands::run;
use crate::config::FuzixConfig;
use crate::toolchain::ToolchainManager;
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

pub fn execute(
    source: Option<PathBuf>,
    args: Vec<String>,
    verbose: bool,
    emulator_override: Option<String>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<()> {
    let src_path = source.unwrap_or_else(|| PathBuf::from(&config.project.source));
    let binary = build::execute(Some(src_path.clone()), None, None, config, toolchain)?;

    let binary_name = binary
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("app");

    let fuzix_dest = format!("/bin/{}", binary_name);
    disk::execute_cp(binary, fuzix_dest.clone(), Some("755".to_string()), config, toolchain)?;

    println!(
        "{} Running test command inside FUZIX: {}",
        "==>".cyan().bold(),
        fuzix_dest.bold()
    );

    run::execute(
        fuzix_dest,
        args,
        verbose,
        emulator_override,
        config,
        toolchain,
    )?;

    println!("{} Test completed successfully", "✓".green().bold());
    Ok(())
}
