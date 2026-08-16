use colored::Colorize;

struct EmulatorInfo {
    name: &'static str,
    cpu: &'static str,
    description: &'static str,
}

const EMULATORS: &[EmulatorInfo] = &[
    EmulatorInfo {
        name: "v68",
        cpu: "68000 (32-bit)",
        description: "Virtual68 platform with software MMU & IDE disk support (Recommended for 68k)",
    },
    EmulatorInfo {
        name: "tiny68k",
        cpu: "68000 (32-bit)",
        description: "Tiny68K Single Board Computer emulation from EmulatorKit",
    },
    EmulatorInfo {
        name: "mini68k",
        cpu: "68000 (32-bit)",
        description: "Mini68K SBC with CompactFlash support",
    },
    EmulatorInfo {
        name: "rc2014",
        cpu: "Z80 (8-bit)",
        description: "RC2014 modular bus retrocomputer (Standard / SC108 / SC114)",
    },
    EmulatorInfo {
        name: "rcbus-z80",
        cpu: "Z80 (8-bit)",
        description: "RCBus standard Z80 modular board",
    },
    EmulatorInfo {
        name: "rcbus-z180",
        cpu: "Z180 (8/16-bit)",
        description: "RCBus Z180 high-speed enhanced Z80 board",
    },
    EmulatorInfo {
        name: "cpmsim",
        cpu: "8080 / Z80 (8-bit)",
        description: "z80pack Cromemco / CP/M system simulator",
    },
    EmulatorInfo {
        name: "swt6809",
        cpu: "6809 (8/16-bit)",
        description: "SWTPC 6809 computer system",
    },
    EmulatorInfo {
        name: "rcbus-6502",
        cpu: "6502 (8-bit)",
        description: "RCBus 6502 processor system",
    },
    EmulatorInfo {
        name: "altair8080",
        cpu: "8080 (8-bit)",
        description: "MITS Altair 8800 S-100 system",
    },
];

pub fn execute() {
    println!("{}", "Available Emulators and Targets:".bold());
    println!("{:<14} {:<20} {}", "EMULATOR".cyan(), "CPU".yellow(), "DESCRIPTION".white());
    println!("{}", "─".repeat(70));

    for emu in EMULATORS {
        println!("{:<14} {:<20} {}", emu.name.bold(), emu.cpu, emu.description);
    }

    println!("\nSet in fuzix.toml:");
    println!("  {}", "[target]".cyan());
    println!("  emulator = \"v68\"");
    println!("  cpu = \"68000\"");
}
