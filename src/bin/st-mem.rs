use std::process::Command;
use st_mem::{MemoryConfig, analyze_elf, analyze_elf_detailed, format_report, format_sections, ExportReport};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_DESC: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_REPO: &str = env!("CARGO_PKG_REPOSITORY");
const PKG_LICENSE: &str = env!("CARGO_PKG_LICENSE");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rest: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    if rest.is_empty() || rest.iter().any(|&a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    if rest.iter().any(|&a| a == "--version" || a == "-V") {
        print_version();
        return;
    }

    if rest[0] == "runner" {
        run_as_runner(&rest[1..]);
        return;
    }

    run_analyze(&rest);
}

fn run_analyze(args: &[&str]) {
    let memory_x = find_arg(args, "--memory-x").unwrap_or_else(|| "memory.x".to_string());
    let elf_path = find_arg(args, "--elf");
    let bar_width: usize = find_arg(args, "--width")
        .and_then(|w| w.parse().ok())
        .unwrap_or(30);
    let show_sections = args.iter().any(|&a| a == "--sections");
    let export = find_arg(args, "--export");

    let elf_path = elf_path
        .or_else(|| args.first().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            eprintln!("[ERROR] No ELF file specified.");
            eprintln!("  Usage: st-mem <elf-path> [--memory-x memory.x]");
            std::process::exit(1);
        });

    let config = MemoryConfig::from_file(&memory_x).unwrap_or_else(|e| {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    });

    if show_sections || export.is_some() {
        let analysis = analyze_elf_detailed(&elf_path, &config).unwrap_or_else(|e| {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        });

        match export.as_deref() {
            Some("json") => {
                let report = ExportReport::from_analysis(&analysis);
                println!("{}", report.to_json());
                return;
            }
            Some("md") | Some("markdown") => {
                let report = ExportReport::from_analysis(&analysis);
                print!("{}", report.to_markdown());
                return;
            }
            Some(other) => {
                eprintln!("[ERROR] Unknown export format '{}'. Use 'json' or 'md'.", other);
                std::process::exit(1);
            }
            None => {}
        }

        println!();
        println!("{}", format_report(&analysis.usage, bar_width));
        if show_sections {
            print!("{}", format_sections(&analysis, bar_width));
        }
        println!();
        println!("[INFO] Firmware: {}", elf_path);
    } else {
        let usage = analyze_elf(&elf_path, &config).unwrap_or_else(|e| {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        });

        println!();
        println!("{}", format_report(&usage, bar_width));
        println!();
        println!("[INFO] Firmware: {}", elf_path);
    }
}

/// Runner 模式: 分析 → 烧录
fn run_as_runner(args: &[&str]) {
    if args.is_empty() {
        eprintln!("[ERROR] runner mode requires an ELF path");
        std::process::exit(1);
    }

    let mut elf_idx = None;
    for (i, &arg) in args.iter().enumerate() {
        if !arg.starts_with('-') {
            elf_idx = Some(i);
        }
    }

    let elf_idx = elf_idx.unwrap_or_else(|| {
        eprintln!("[ERROR] No ELF file found in arguments");
        std::process::exit(1);
    });

    let elf_path = args[elf_idx];
    let probe_rs_args = args;
    let memory_x = find_arg(args, "--memory-x").unwrap_or_else(|| "memory.x".to_string());
    let show_sections = args.iter().any(|&a| a == "--sections");

    match MemoryConfig::from_file(&memory_x) {
        Ok(config) => {
            if show_sections {
                if let Ok(analysis) = analyze_elf_detailed(elf_path, &config) {
                    println!();
                    println!("{}", format_report(&analysis.usage, 30));
                    print!("{}", format_sections(&analysis, 30));
                    println!();
                    println!("[INFO] Firmware: {}", elf_path);
                }
            } else if let Ok(usage) = analyze_elf(elf_path, &config) {
                println!();
                println!("{}", format_report(&usage, 30));
                println!();
                println!("[INFO] Firmware: {}", elf_path);
            }
        }
        Err(e) => {
            eprintln!("[WARN] {}: skip memory analysis", e);
        }
    }

    run_probe_rs(probe_rs_args);
}

fn run_probe_rs(args: &[&str]) {
    println!("[FLASH] Programming via probe-rs...");
    println!();

    let status = Command::new("probe-rs")
        .arg("run")
        .args(args)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!();
            println!("[DONE] Flash complete!");
        }
        Ok(s) => {
            eprintln!("[ERROR] probe-rs exited with: {:?}", s);
            std::process::exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("[ERROR] Failed to execute probe-rs: {}", e);
            eprintln!("  Make sure probe-rs is installed: cargo install probe-rs-tools");
            std::process::exit(1);
        }
    }
}

fn find_arg(args: &[&str], name: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == name && i + 1 < args.len() {
            return Some(args[i + 1].to_string());
        }
    }
    None
}

fn print_version() {
    println!("{} v{}", PKG_NAME, VERSION);
    println!("{}", PKG_DESC);
    println!("Author: {}", PKG_AUTHORS);
    println!("License: {}", PKG_LICENSE);
    println!("Repository: {}", PKG_REPO);
}

fn print_help() {
    println!("st-mem v{} — Analyze embedded firmware memory usage & flash tool", VERSION);
    println!();
    println!("Usage:");
    println!("  st-mem <elf-path> [OPTIONS]         Analyze firmware memory");
    println!("  st-mem runner <probe-rs-args>       Analyze + flash");
    println!();
    println!("Options:");
    println!("  --memory-x <path>   Path to memory.x linker script (default: memory.x)");
    println!("  --elf <path>        Path to ELF binary (or use positional arg)");
    println!("  --width <n>         Progress bar width in characters (default: 30)");
    println!("  --sections          Show per-section breakdown (default: off)");
    println!("  --export <fmt>      Export report as 'json' or 'md' (default: off)");
    println!("  --version, -V       Show version and project info");
    println!("  --help, -h          Show this help");
    println!();
    println!("Examples:");
    println!("  st-mem target/thumbv7m-none-eabi/debug/firmware");
    println!("  st-mem firmware.elf --sections");
    println!("  st-mem firmware.elf --export json > report.json");
    println!("  st-mem firmware.elf --export md > report.md");
    println!("  st-mem runner --chip STM32F103C8 --protocol swd firmware.elf");
}
