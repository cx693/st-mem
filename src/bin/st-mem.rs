use std::process::Command;
use st_mem::{MemoryConfig, analyze_elf, analyze_elf_detailed, format_report, format_sections, ExportReport};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_DESC: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_REPO: &str = env!("CARGO_PKG_REPOSITORY");
const PKG_LICENSE: &str = env!("CARGO_PKG_LICENSE");

/// st-mem 自身的参数集合
struct StMemOpts {
    memory_x: String,
    bar_width: usize,
    show_sections: bool,
    export: Option<String>,
}

impl Default for StMemOpts {
    fn default() -> Self {
        StMemOpts {
            memory_x: "memory.x".to_string(),
            bar_width: 30,
            show_sections: false,
            export: None,
        }
    }
}

fn parse_st_mem_opts(args: &[&str]) -> StMemOpts {
    let mut opts = StMemOpts::default();
    if let Some(v) = find_arg(args, "--memory-x") { opts.memory_x = v; }
    if let Some(v) = find_arg(args, "--width") {
        opts.bar_width = v.parse().unwrap_or(30);
    }
    if args.iter().any(|&a| a == "--sections") { opts.show_sections = true; }
    if let Some(v) = find_arg(args, "--export") { opts.export = Some(v); }
    opts
}

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

    // 查找 "runner" 关键字的位置
    let runner_pos = rest.iter().position(|&a| a == "runner");

    if let Some(pos) = runner_pos {
        // runner 模式: runner 之前的是 st-mem 参数，之后的是 probe-rs 参数
        let st_mem_args = &rest[..pos];
        let probe_rs_args = &rest[pos + 1..];
        let opts = parse_st_mem_opts(st_mem_args);
        run_as_runner(probe_rs_args, &opts);
        return;
    }

    run_analyze(&rest);
}

fn run_analyze(args: &[&str]) {
    let opts = parse_st_mem_opts(args);

    let elf_path = find_arg(args, "--elf")
        .or_else(|| {
            // 取第一个不以 -- 开头且不是某个 flag 的值的参数
            let mut i = 0;
            while i < args.len() {
                if args[i].starts_with('-') {
                    // 跳过 flag 及其可能的值
                    if matches!(args[i], "--memory-x" | "--elf" | "--width" | "--export") {
                        i += 2; // 跳过 flag + value
                        continue;
                    }
                    i += 1;
                    continue;
                }
                return Some(args[i].to_string());
            }
            None
        })
        .unwrap_or_else(|| {
            eprintln!("[ERROR] No ELF file specified.");
            eprintln!("  Usage: st-mem <elf-path> [--memory-x memory.x]");
            std::process::exit(1);
        });

    let config = MemoryConfig::from_file(&opts.memory_x).unwrap_or_else(|e| {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    });

    do_analysis(&elf_path, &config, &opts);
}

/// Runner 模式: 分析 → 烧录
///
/// 命令格式:
///   st-mem [st-mem flags] runner [probe-rs args] <elf-path>
///
/// 例如:
///   st-mem --sections runner --chip STM32F103C8 --protocol swd firmware.elf
///   st-mem runner --chip STM32F103C8 --protocol swd firmware.elf
fn run_as_runner(probe_rs_args: &[&str], opts: &StMemOpts) {
    if probe_rs_args.is_empty() {
        eprintln!("[ERROR] runner mode requires probe-rs arguments and an ELF path");
        eprintln!("  Usage: st-mem [st-mem-flags] runner [probe-rs-args] <elf-path>");
        std::process::exit(1);
    }

    // 从 probe_rs_args 中找 ELF 路径（最后一个不以 - 开头的参数）
    let mut elf_idx = None;
    for (i, &arg) in probe_rs_args.iter().enumerate() {
        if !arg.starts_with('-') {
            elf_idx = Some(i);
        }
    }

    let elf_idx = elf_idx.unwrap_or_else(|| {
        eprintln!("[ERROR] No ELF file found in runner arguments");
        std::process::exit(1);
    });

    let elf_path = probe_rs_args[elf_idx];

    match MemoryConfig::from_file(&opts.memory_x) {
        Ok(config) => {
            if let Ok(analysis) = analyze_elf_detailed(elf_path, &config) {
                println!();
                println!("{}", format_report(&analysis.usage, opts.bar_width));
                if opts.show_sections {
                    print!("{}", format_sections(&analysis, opts.bar_width));
                }
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

/// 分析 ELF 并根据 opts 决定输出格式
fn do_analysis(elf_path: &str, config: &MemoryConfig, opts: &StMemOpts) {
    if opts.show_sections || opts.export.is_some() {
        let analysis = analyze_elf_detailed(elf_path, config).unwrap_or_else(|e| {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        });

        match opts.export.as_deref() {
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
        println!("{}", format_report(&analysis.usage, opts.bar_width));
        if opts.show_sections {
            print!("{}", format_sections(&analysis, opts.bar_width));
        }
        println!();
        println!("[INFO] Firmware: {}", elf_path);
    } else {
        let usage = analyze_elf(elf_path, config).unwrap_or_else(|e| {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        });

        println!();
        println!("{}", format_report(&usage, opts.bar_width));
        println!();
        println!("[INFO] Firmware: {}", elf_path);
    }
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
    println!("  st-mem <elf-path> [OPTIONS]                           Analyze firmware memory");
    println!("  st-mem [st-mem-OPTIONS] runner [probe-rs-args]        Analyze + flash");
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
    println!("Runner mode:");
    println!("  st-mem flags are placed BEFORE 'runner', probe-rs flags AFTER.");
    println!("  Example:");
    println!("    st-mem --sections --width 40 runner --chip STM32F103C8 --protocol swd firmware.elf");
    println!();
    println!("Examples:");
    println!("  st-mem target/thumbv7m-none-eabi/debug/firmware");
    println!("  st-mem firmware.elf --sections");
    println!("  st-mem firmware.elf --export json > report.json");
    println!("  st-mem firmware.elf --export md > report.md");
    println!("  st-mem runner --chip STM32F103C8 --protocol swd firmware.elf");
    println!("  st-mem --sections --width 40 runner --chip STM32F103C8 --protocol swd firmware.elf");
}
