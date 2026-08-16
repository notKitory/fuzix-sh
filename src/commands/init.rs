use crate::config::{FuzixConfig, CONFIG_FILE_NAME};
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn execute(name: Option<String>, cpu: Option<String>, emulator: Option<String>) -> Result<()> {
    let config_path = Path::new(CONFIG_FILE_NAME);
    if config_path.exists() {
        bail!("{} already exists in this directory.", CONFIG_FILE_NAME);
    }

    let mut config = FuzixConfig::default();
    if let Some(n) = name {
        config.project.name = n;
    }
    if let Some(c) = cpu {
        config.target.cpu = c;
    }
    if let Some(e) = emulator {
        config.target.emulator = e;
    }

    config.save(config_path)?;

    // Create a sample hello.c if none exists
    let hello_path = Path::new("hello.c");
    if !hello_path.exists() {
        let sample_c = r#"#include <stdio.h>

int main(int argc, char **argv) {
    printf("Hello from FUZIX OS!\n");
    for (int i = 1; i < argc; i++) {
        printf("arg[%d] = %s\n", i, argv[i]);
    }
    return 0;
}
"#;
        fs::write(hello_path, sample_c)?;
    }

    println!(
        "{} Initialized new FUZIX project in {}",
        "✓".green().bold(),
        config_path.display().to_string().bold()
    );
    println!("  Target CPU:      {}", config.target.cpu.cyan());
    println!("  Target Emulator: {}", config.target.emulator.cyan());
    println!("\nNext steps:");
    println!("  {} fuzix build hello.c", "•".bold());
    println!("  {} fuzix test hello.c", "•".bold());
    println!("  {} fuzix shell", "•".bold());

    Ok(())
}
