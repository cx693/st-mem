use crate::elf::FirmwareUsage;

/// Generate a text-based progress bar.
///
/// # Arguments
///
/// * `pct` - Percentage (0.0 - 100.0).
/// * `width` - Number of characters in the bar.
///
/// # Returns
///
/// A string like `[████████░░░░░░░░]`.
pub fn progress_bar(pct: f64, width: usize) -> String {
    let fill = if pct <= 0.0 {
        0
    } else {
        let f = (pct / 100.0 * width as f64) as usize;
        if f < 1 { 1 } else { f }.min(width)
    };
    let empty = width - fill;
    format!("[{}{}]", "\u{2588}".repeat(fill), "\u{2591}".repeat(empty))
}

/// Format byte count into a human-readable string (B, KB, MB).
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{} MB", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}

/// Generate a full firmware memory report as a formatted string.
///
/// Produces output like:
/// ```text
///   [FIRMWARE SIZE]
///   +----------------------------------------------------------------+
///   | FLASH [████░░░░░░░░░░░░░░░░░░░░░░░░░░]  15.1%    9.7K / 64K    |
///   | RAM   [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0.0%      4B / 20K    |
///   +----------------------------------------------------------------+
/// ```
///
/// # Arguments
///
/// * `usage` - The firmware usage data.
/// * `bar_width` - Number of characters for the progress bar (default 30).
pub fn format_report(usage: &FirmwareUsage, bar_width: usize) -> String {
    format_report_with_labels(usage, bar_width, "FLASH", "RAM")
}

/// Generate a firmware memory report with custom region labels.
pub fn format_report_with_labels(
    usage: &FirmwareUsage,
    bar_width: usize,
    flash_label: &str,
    ram_label: &str,
) -> String {
    let flash_bar = progress_bar(usage.flash_percent(), bar_width);
    let ram_bar = progress_bar(usage.ram_percent(), bar_width);

    let flash_used_str = format_bytes(usage.flash_used);
    let flash_total_str = format_bytes(usage.flash_total);
    let ram_used_str = format_bytes(usage.ram_used);
    let ram_total_str = format_bytes(usage.ram_total);

    // Calculate the maximum label width
    let label_width = flash_label.len().max(ram_label.len());

    // Format each line
    let line_flash = format!(
        " {:<label$} {} {:>5.1}%  {:>6} / {:<6} ",
        flash_label,
        flash_bar,
        usage.flash_percent(),
        flash_used_str,
        flash_total_str,
        label = label_width
    );
    let line_ram = format!(
        " {:<label$} {} {:>5.1}%  {:>6} / {:<6} ",
        ram_label,
        ram_bar,
        usage.ram_percent(),
        ram_used_str,
        ram_total_str,
        label = label_width
    );

    // Determine border width from the longest line (character count)
    let total_width = line_flash.chars().count().max(line_ram.chars().count());
    let border = format!("+{}+", "-".repeat(total_width));

    // Pad lines to fixed width
    let pad = |s: &str, w: usize| {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() >= w {
            chars[..w].iter().collect()
        } else {
            format!("{}{}", s, " ".repeat(w - chars.len()))
        }
    };

    let mut out = String::new();
    out.push_str("  [FIRMWARE SIZE]\n");
    out.push_str(&format!("  {}\n", border));
    out.push_str(&format!("  |{}|\n", pad(&line_flash, total_width)));
    out.push_str(&format!("  |{}|\n", pad(&line_ram, total_width)));
    out.push_str(&format!("  {}", border));
    out
}

/// Convenience: analyze and print a firmware memory report to stdout.
///
/// # Arguments
///
/// * `elf_path` - Path to the ELF binary.
/// * `memory_x_path` - Path to the `memory.x` linker script.
pub fn print_report<P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
    elf_path: P1,
    memory_x_path: P2,
) -> Result<(), String> {
    let config = crate::memory::MemoryConfig::from_file(memory_x_path)?;
    let usage = crate::elf::analyze_elf(elf_path, &config)?;
    println!("{}", format_report(&usage, 30));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0.0, 10), "[░░░░░░░░░░]");
        assert_eq!(progress_bar(100.0, 10), "[██████████]");
        assert_eq!(progress_bar(50.0, 10), "[█████░░░░░]");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(65536), "64 KB");
        assert_eq!(format_bytes(1048576), "1 MB");
    }

    #[test]
    fn test_format_report() {
        let usage = FirmwareUsage {
            flash_used: 9933,
            ram_used: 4,
            flash_total: 65536,
            ram_total: 20480,
        };
        let report = format_report(&usage, 30);
        println!("{}", report);
        assert!(report.contains("FIRMWARE SIZE"));
        assert!(report.contains("FLASH"));
        assert!(report.contains("RAM"));
    }
}
