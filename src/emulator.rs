use crate::config::FuzixConfig;
use crate::disk::DiskManager;
use crate::toolchain::ToolchainManager;
use anyhow::{bail, Result};
use colored::Colorize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct EmulatorRunner<'a> {
    pub config: &'a FuzixConfig,
    pub toolchain: &'a ToolchainManager,
    pub disk: &'a DiskManager<'a>,
}

impl<'a> EmulatorRunner<'a> {
    pub fn new(config: &'a FuzixConfig, toolchain: &'a ToolchainManager, disk: &'a DiskManager<'a>) -> Self {
        Self { config, toolchain, disk }
    }

    pub fn prepare_emulator_working_dir(&self, emulator: &str) -> Result<PathBuf> {
        let work_dir = PathBuf::from(".fuzix").join("emulator_work");
        fs::create_dir_all(&work_dir)?;

        self.disk.ensure_images()?;
        let root = if self.disk.root_image_path().exists() {
            fs::canonicalize(self.disk.root_image_path())?
        } else {
            self.disk.root_image_path()
        };
        let boot = if self.disk.boot_image_path().exists() {
            fs::canonicalize(self.disk.boot_image_path()).unwrap_or_else(|_| self.disk.boot_image_path())
        } else {
            self.disk.boot_image_path()
        };

        if emulator == "68knano" {
            let emu_dir = self.toolchain.emulators_dir();
            let rom_path = emu_dir.join("68knano.rom");
            if rom_path.exists() {
                let _ = fs::copy(&rom_path, work_dir.join("68knano.rom"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                let ide_path = work_dir.join("68knano.ide");
                let _ = fs::remove_file(&ide_path);
                let _ = symlink(&root, &ide_path);
            }
        } else if emulator == "v68" || emulator == "tiny68k" {
            let emu_dir = self.toolchain.emulators_dir();
            let boot_dat = emu_dir.join("boot.dat");
            if boot_dat.exists() {
                let _ = fs::copy(&boot_dat, work_dir.join("boot.dat"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                let disk_img = work_dir.join("disk.img");
                let drive_ide = work_dir.join("drive.ide");
                let _ = fs::remove_file(&disk_img);
                let _ = fs::remove_file(&drive_ide);
                let _ = symlink(&root, &disk_img);
                let _ = symlink(&root, &drive_ide);
            }
        } else if emulator == "cpmsim" {
            let disks_dir = work_dir.join("disks");
            fs::create_dir_all(&disks_dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                let _ = fs::remove_file(disks_dir.join("drivea.dsk"));
                let _ = fs::remove_file(disks_dir.join("drivei.dsk"));
                symlink(&boot, disks_dir.join("drivea.dsk"))?;
                symlink(&root, disks_dir.join("drivei.dsk"))?;
            }
        }

        Ok(work_dir)
    }

    fn build_command(&self, emulator: &str, work_dir: &Path) -> CommandBuilder {
        let emu_bin = self.toolchain.emulator_binary(emulator);
        let mut cmd = CommandBuilder::new(&emu_bin);
        cmd.cwd(work_dir);

        if emulator == "68knano" {
            cmd.arg("-f");
            if let Some(c) = self.config.target.cpu.strip_prefix("680") {
                match c {
                    "10" => { cmd.arg("-1"); }
                    "20" => { cmd.arg("-2"); }
                    _ => { cmd.arg("-0"); }
                }
            }
            if work_dir.join("68knano.rom").exists() {
                cmd.arg("-r");
                cmd.arg("68knano.rom");
            }
            if work_dir.join("68knano.ide").exists() {
                cmd.arg("-i");
                cmd.arg("68knano.ide");
            }
        }

        cmd
    }

    /// Run an interactive shell in the emulator.
    pub fn shell(&self, emulator_override: Option<&str>) -> Result<()> {
        let emu_name = emulator_override.unwrap_or(&self.config.target.emulator);
        let emu_bin = self.toolchain.emulator_binary(emu_name);

        if !emu_bin.exists() {
            bail!("Emulator binary not found: {}", emu_bin.display());
        }

        let work_dir = self.prepare_emulator_working_dir(emu_name)?;

        println!(
            "{} Starting interactive session on {} (Press Ctrl-] or type 'shutdown' to exit)",
            "==>".bright_green().bold(),
            emu_name.bold()
        );

        let pty_system = native_pty_system();
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let cmd = self.build_command(emu_name, &work_dir);

        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;

        enable_raw_mode()?;
        struct RawModeGuard;
        impl Drop for RawModeGuard {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }
        let _guard = RawModeGuard;

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Background thread to stream emulator output to stdout
        let stdout_thread = std::thread::spawn(move || {
            let mut stdout = std::io::stdout();
            let mut buf = [0u8; 1024];
            while running_clone.load(Ordering::Relaxed) {
                if let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let _ = stdout.write_all(&buf[..n]);
                    let _ = stdout.flush();
                } else {
                    break;
                }
            }
        });

        // Main thread streams stdin to emulator
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 128];
        while running.load(Ordering::Relaxed) {
            if let Ok(n) = stdin.read(&mut buf) {
                if n == 0 {
                    break;
                }
                // Check for Ctrl-] (ASCII 29 / 0x1D)
                if buf[..n].contains(&0x1D) {
                    println!("\r\n[Force terminating emulator...]");
                    let _ = child.kill();
                    break;
                }
                if writer.write_all(&buf[..n]).is_err() {
                    break;
                }
            } else {
                break;
            }
        }

        running.store(false, Ordering::Relaxed);
        let _ = child.wait();
        let _ = stdout_thread.join();

        println!("\r\n{} Emulator session closed.", "✓".green());
        Ok(())
    }

    /// Run a command in the emulator non-interactively and capture output.
    pub fn run_command(
        &self,
        command_line: &str,
        verbose: bool,
        timeout_secs: Option<u64>,
        emulator_override: Option<&str>,
    ) -> Result<String> {
        let emu_name = emulator_override.unwrap_or(&self.config.target.emulator);
        let emu_bin = self.toolchain.emulator_binary(emu_name);

        if !emu_bin.exists() {
            bail!("Emulator binary not found: {}", emu_bin.display());
        }

        let work_dir = self.prepare_emulator_working_dir(emu_name)?;
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(self.config.target.timeout));

        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let cmd = self.build_command(emu_name, &work_dir);

        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;

        let mut captured_output = String::new();
        let start_time = Instant::now();

        let mut logged_in = false;
        let mut command_sent = false;
        let mut command_finished = false;

        let mut read_buf = [0u8; 1024];

        while start_time.elapsed() < timeout {
            if let Ok(n) = reader.read(&mut read_buf) {
                if n > 0 {
                    let chunk = String::from_utf8_lossy(&read_buf[..n]);
                    captured_output.push_str(&chunk);

                    if verbose {
                        print!("{}", chunk);
                        let _ = std::io::stdout().flush();
                    }

                    // Handle boot prompting for v68 or cpmsim
                    if !logged_in {
                        if chunk.contains("bootdev:") {
                            let _ = writer.write_all(b"0\r");
                        } else if chunk.contains("boot:") || chunk.contains("Boot>") {
                            let _ = writer.write_all(b"b\r");
                        } else if chunk.contains("login:") {
                            let _ = writer.write_all(b"root\r");
                        } else if chunk.contains("# ") || chunk.contains("$ ") {
                            logged_in = true;
                        }
                    }

                    // If logged in and ready, send target command
                    if logged_in && !command_sent {
                        let _ = writer.write_all(format!("{}\r", command_line).as_bytes());
                        command_sent = true;
                        continue;
                    }

                    // If command was sent and we see the prompt again, command has completed!
                    if command_sent && !command_finished {
                        // Check if prompt appears after the command echo
                        if captured_output.contains("# shutdown") || captured_output.ends_with("# ") || captured_output.ends_with("$ ") {
                            let _ = writer.write_all(b"shutdown\r");
                            command_finished = true;
                        }
                    }

                    if command_finished && (chunk.contains("Halted") || chunk.contains("System halted") || chunk.contains("halt:")) {
                        break;
                    }
                }
            } else {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let _ = child.kill();
        let _ = child.wait();

        if !command_sent {
            bail!("Timeout waiting for emulator login prompt");
        }

        // Clean output to extract only the program result
        let clean_output = extract_program_output(&captured_output, command_line);
        Ok(clean_output)
    }
}

fn extract_program_output(full_log: &str, command: &str) -> String {
    let mut lines = full_log.lines();
    let mut collecting = false;
    let mut result_lines = Vec::new();

    while let Some(line) = lines.next() {
        if !collecting {
            if line.contains(command) {
                collecting = true;
            }
        } else {
            if line.contains("shutdown") || line.starts_with("# ") || line.starts_with("$ ") || line.contains("System halted") || line.contains("Halted") {
                break;
            }
            result_lines.push(line);
        }
    }

    result_lines.join("\n")
}
