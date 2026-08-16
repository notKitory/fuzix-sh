use crate::config::FuzixConfig;
use crate::toolchain::ToolchainManager;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Compiler<'a> {
    pub config: &'a FuzixConfig,
    pub toolchain: &'a ToolchainManager,
}

impl<'a> Compiler<'a> {
    pub fn new(config: &'a FuzixConfig, toolchain: &'a ToolchainManager) -> Self {
        Self { config, toolchain }
    }

    /// Compile a C source file into a FUZIX binary.
    pub fn compile_source<P: AsRef<Path>>(&self, source: P, output: Option<PathBuf>) -> Result<PathBuf> {
        let src_path = source.as_ref();
        if !src_path.exists() {
            bail!("Source file not found: {}", src_path.display());
        }

        let file_stem = src_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("app");

        let out_dir = PathBuf::from(".fuzix").join("bin");
        fs::create_dir_all(&out_dir)?;

        let final_out = output.unwrap_or_else(|| out_dir.join(file_stem));
        let build_dir = PathBuf::from(".fuzix").join("build").join(file_stem);
        fs::create_dir_all(&build_dir)?;

        let fcc = self.toolchain.fcc_binary();
        let runtime = self.toolchain.runtime_dir();
        let cpu = &self.config.target.cpu;

        println!(
            "{} Compiling {} for CPU {}",
            "==>".cyan().bold(),
            src_path.display().to_string().bold(),
            cpu.yellow().bold()
        );

        let obj_file = build_dir.join(format!("{}.o", file_stem));
        let b1_file = build_dir.join(format!("{}.b1", file_stem));

        // Step 1: Compile .c to .o
        let mut cmd = Command::new(&fcc);
        cmd.env("FUZIX_ROOT", &runtime)
            .arg(format!("-m{}", cpu))
            .arg("-Os")
            .arg("-D__STDC__")
            .arg("-c")
            .arg(format!("-I{}", runtime.join("include").display()))
            .arg(format!("-I{}", runtime.join("include").join(cpu).display()))
            .arg(src_path)
            .arg("-o")
            .arg(&obj_file);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to run compiler: {}", fcc.display()))?;

        if !status.success() {
            bail!("Compilation failed.");
        }

        // Step 2: Link binary
        let crt0 = runtime.join("libs").join(format!("crt0_{}.o", cpu));
        let mut link_cmd = Command::new(&fcc);
        link_cmd
            .env("FUZIX_ROOT", &runtime)
            .arg("-s")
            .arg(format!("-m{}", cpu));

        if crt0.exists() {
            link_cmd.arg(&crt0);
        }

        link_cmd
            .arg(&obj_file)
            .arg("-o")
            .arg(&b1_file)
            .arg("-M")
            .arg(format!("-L{}", runtime.join("libs").display()))
            .arg(format!("-lc{}", cpu))
            .arg(format!("-lc{}", cpu));

        let link_status = link_cmd.status().context("Failed to run linker")?;
        if !link_status.success() {
            bail!("Linking failed.");
        }

        // Step 3: Run binman if available for 8-bit targets
        let binman = runtime.join("tools").join("binman85");
        fs::copy(&b1_file, &final_out)?;

        if (cpu == "8080" || cpu == "8085" || cpu == "z80") && binman.exists() {
            let _ = Command::new(&binman).arg(&final_out).status();
        }

        println!(
            "{} Built target successfully: {}",
            "✓".green().bold(),
            final_out.display().to_string().bold()
        );

        Ok(final_out)
    }

    /// Invoke `make` with FUZIX environment set.
    pub fn make(&self, targets: &[String]) -> Result<()> {
        let fcc = self.toolchain.fcc_binary();
        let runtime = self.toolchain.runtime_dir();
        let fuzix_root = runtime.join("fuzix");
        let cpu = &self.config.target.cpu;

        let mut cmd = Command::new("make");
        cmd.env("FUZIX_ROOT", &fuzix_root)
            .env("FUZIX_CPU", cpu)
            .env("FUZIX_CC", &fcc)
            .env(
                "FUZIX_CFLAGS",
                format!(
                    "-m{} -Os -D__STDC__ -I{} -I{}",
                    cpu,
                    fuzix_root.join("include").display(),
                    fuzix_root.join("include").join(cpu).display()
                ),
            )
            .env("FUZIX_LDFLAGS", format!("-m{} -L{}", cpu, fuzix_root.join("libs").display()))
            .env("FUZIX_CRT0", fuzix_root.join("libs").join(format!("crt0_{}.o", cpu)))
            .env("FUZIX_LIBS", format!("-lc{} -lc{}", cpu, cpu));

        for target in targets {
            cmd.arg(target);
        }

        let status = cmd.status().context("Failed to run make")?;
        if !status.success() {
            bail!("Make failed with exit code {}", status);
        }

        Ok(())
    }
}
