mod commands;
mod compiler;
mod config;
mod disk;
mod emulator;
mod toolchain;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::FuzixConfig;
use std::path::PathBuf;
use toolchain::ToolchainManager;

#[derive(Parser)]
#[command(
    name = "fuzix",
    author = "kitory",
    version = "0.2.0",
    about = "Modern native CLI developer tool and SDK for FUZIX OS"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new FUZIX project in the current directory
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
        /// Target CPU (e.g. 68000, 8080, z80, 6809)
        #[arg(short, long)]
        cpu: Option<String>,
        /// Target emulator (e.g. v68, tiny68k, rc2014, cpmsim)
        #[arg(short, long)]
        emulator: Option<String>,
    },

    /// Compile a C source file or run make
    Build {
        /// C source file to compile
        #[arg(value_name = "SOURCE.c")]
        source: Option<PathBuf>,
        /// Output binary path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Run make targets instead
        #[arg(long, num_args = 0..)]
        make: Option<Vec<String>>,
    },

    /// Manage files on the FUZIX root disk image
    Disk {
        #[command(subcommand)]
        sub: DiskSubcommands,
    },

    /// Run a command inside the FUZIX emulator
    Run {
        /// Command to execute inside FUZIX
        command: String,
        /// Command arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Show full emulator startup log
        #[arg(short, long)]
        verbose: bool,
        /// Override emulator
        #[arg(short, long)]
        emulator: Option<String>,
    },

    /// Build, copy to disk, and run test in sequence
    Test {
        /// Source file to test
        #[arg(value_name = "SOURCE.c")]
        source: Option<PathBuf>,
        /// Program arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Show full emulator output
        #[arg(short, long)]
        verbose: bool,
        /// Override emulator
        #[arg(short, long)]
        emulator: Option<String>,
    },

    /// Open an interactive shell inside FUZIX
    Shell {
        /// Override emulator
        #[arg(short, long)]
        emulator: Option<String>,
    },

    /// List supported emulators and targets
    Emulators,
}

#[derive(Subcommand)]
enum DiskSubcommands {
    /// Copy a host file into the FUZIX root disk
    Cp {
        /// Local host file path
        src: PathBuf,
        /// Destination path on FUZIX disk (e.g. /bin/app)
        dest: String,
        /// Optional chmod mode (e.g. 755)
        #[arg(short, long)]
        mode: Option<String>,
    },
    /// List files on the FUZIX disk
    Ls {
        /// Target directory on FUZIX disk (default: /bin)
        path: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (_, config) = FuzixConfig::load_or_default(None);
    let toolchain = ToolchainManager::new(&config.toolchain.repo, &config.toolchain.release)?;

    match cli.command {
        Commands::Init { name, cpu, emulator } => {
            commands::init::execute(name, cpu, emulator)?;
        }
        Commands::Build { source, output, make } => {
            commands::build::execute(source, output, make, &config, &toolchain)?;
        }
        Commands::Disk { sub } => match sub {
            DiskSubcommands::Cp { src, dest, mode } => {
                commands::disk::execute_cp(src, dest, mode, &config, &toolchain)?;
            }
            DiskSubcommands::Ls { path } => {
                commands::disk::execute_ls(path, &config, &toolchain)?;
            }
        },
        Commands::Run { command, args, verbose, emulator } => {
            commands::run::execute(command, args, verbose, emulator, &config, &toolchain)?;
        }
        Commands::Test { source, args, verbose, emulator } => {
            commands::test::execute(source, args, verbose, emulator, &config, &toolchain)?;
        }
        Commands::Shell { emulator } => {
            commands::shell::execute(emulator, &config, &toolchain)?;
        }
        Commands::Emulators => {
            commands::emulators::execute();
        }
    }

    Ok(())
}
