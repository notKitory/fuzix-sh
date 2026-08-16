use crate::compiler::Compiler;
use crate::config::FuzixConfig;
use crate::toolchain::ToolchainManager;
use anyhow::Result;
use std::path::PathBuf;

pub fn execute(
    source: Option<PathBuf>,
    output: Option<PathBuf>,
    make_targets: Option<Vec<String>>,
    config: &FuzixConfig,
    toolchain: &ToolchainManager,
) -> Result<PathBuf> {
    toolchain.ensure_installed()?;
    let compiler = Compiler::new(config, toolchain);

    if let Some(targets) = make_targets {
        compiler.make(&targets)?;
        Ok(PathBuf::from(targets.first().cloned().unwrap_or_else(|| "app".to_string())))
    } else {
        let src = source.unwrap_or_else(|| PathBuf::from(&config.project.source));
        compiler.compile_source(src, output)
    }
}
