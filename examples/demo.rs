use st_mem::{MemoryConfig, analyze_elf, format_report};

fn main() {
    let memory_x = "./memory.x";
    let elf_path = "./stm32dome";

    println!("[INFO] Parsing memory.x: {}", memory_x);
    let config = MemoryConfig::from_file(memory_x).expect("Failed to parse memory.x");

    if let Some(flash) = config.flash() {
        println!("[INFO] FLASH: origin=0x{:08X}, length={}", flash.origin, flash.length);
    }
    if let Some(ram) = config.ram() {
        println!("[INFO] RAM:   origin=0x{:08X}, length={}", ram.origin, ram.length);
    }

    println!();
    println!("[INFO] Analyzing ELF: {}", elf_path);
    let usage = analyze_elf(elf_path, &config).expect("Failed to analyze ELF");

    println!("[INFO] FLASH used: {} bytes", usage.flash_used);
    println!("[INFO] RAM used:   {} bytes", usage.ram_used);
    println!();

    println!("{}", format_report(&usage, 30));
    println!();
    println!("[INFO] Firmware: {}", elf_path);
}
