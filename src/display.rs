use serde::Serialize;

use crate::elf::{ElfAnalysis, FirmwareUsage, RegionType, SectionInfo};

/// Generate a text-based progress bar.
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

    let label_width = flash_label.len().max(ram_label.len());

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

    let total_width = line_flash.chars().count().max(line_ram.chars().count());
    let border = format!("+{}+", "-".repeat(total_width));

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

/// Format a per-section breakdown table.
///
/// Shows each allocated section with its name, region type, size,
/// percentage of region, and a small progress bar.
pub fn format_sections(analysis: &ElfAnalysis, bar_width: usize) -> String {
    let mut out = String::new();

    let flash_sections: Vec<&SectionInfo> = analysis.sections.iter()
        .filter(|s| s.region == RegionType::Flash)
        .collect();
    let ram_sections: Vec<&SectionInfo> = analysis.sections.iter()
        .filter(|s| s.region == RegionType::Ram)
        .collect();

    // Find the longest section name for alignment
    let max_name = analysis.sections.iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(8)
        .max(8);

    if !flash_sections.is_empty() {
        out.push_str(&format!("\n  [FLASH Sections] ({} total)\n", format_bytes(analysis.usage.flash_used)));
        out.push_str(&format!("  {:<name$}  {:>8}  {:>6}  {}\n",
            "NAME", "SIZE", "%", "BAR", name = max_name));
        for sec in &flash_sections {
            let pct = if analysis.usage.flash_total == 0 { 0.0 }
                else { sec.size as f64 * 100.0 / analysis.usage.flash_total as f64 };
            let bar = progress_bar(pct, bar_width);
            out.push_str(&format!("  {:<name$}  {:>8}  {:>5.1}%  {}\n",
                sec.name, format_bytes(sec.size as u64), pct, bar, name = max_name));
        }
    }

    if !ram_sections.is_empty() {
        out.push_str(&format!("\n  [RAM Sections] ({} total)\n", format_bytes(analysis.usage.ram_used)));
        out.push_str(&format!("  {:<name$}  {:>8}  {:>6}  {}\n",
            "NAME", "SIZE", "%", "BAR", name = max_name));
        for sec in &ram_sections {
            let pct = if analysis.usage.ram_total == 0 { 0.0 }
                else { sec.size as f64 * 100.0 / analysis.usage.ram_total as f64 };
            let bar = progress_bar(pct, bar_width);
            out.push_str(&format!("  {:<name$}  {:>8}  {:>5.1}%  {}\n",
                sec.name, format_bytes(sec.size as u64), pct, bar, name = max_name));
        }
    }

    out
}

/// Convenience: analyze and print a firmware memory report to stdout.
pub fn print_report<P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
    elf_path: P1,
    memory_x_path: P2,
) -> Result<(), String> {
    let config = crate::memory::MemoryConfig::from_file(memory_x_path)?;
    let usage = crate::elf::analyze_elf(elf_path, &config)?;
    println!("{}", format_report(&usage, 30));
    Ok(())
}

// ── Export structures ──────────────────────────────────────────────────────

/// Export-friendly metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ExportMeta {
    pub tool: String,
    pub version: String,
    pub repository: String,
}

/// Full report structure for JSON / Markdown export.
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub meta: ExportMeta,
    pub flash_used: u64,
    pub flash_total: u64,
    pub flash_percent: f64,
    pub ram_used: u64,
    pub ram_total: u64,
    pub ram_percent: f64,
    pub sections: Vec<ExportSection>,
}

/// A single section entry for export.
#[derive(Debug, Clone, Serialize)]
pub struct ExportSection {
    pub name: String,
    pub address: String,
    pub size: u32,
    pub region: String,
    pub flags: Vec<String>,
}

impl ExportReport {
    pub fn from_analysis(analysis: &ElfAnalysis) -> Self {
        let sections = analysis.sections.iter().map(|s| {
            let mut flags = Vec::new();
            if s.is_alloc() { flags.push("ALLOC".to_string()); }
            if s.is_writable() { flags.push("WRITE".to_string()); }
            if s.is_executable() { flags.push("EXEC".to_string()); }
            ExportSection {
                name: s.name.clone(),
                address: format!("0x{:08X}", s.address),
                size: s.size,
                region: match s.region {
                    RegionType::Flash => "FLASH",
                    RegionType::Ram => "RAM",
                    RegionType::Other => "OTHER",
                }.to_string(),
                flags,
            }
        }).collect();

        ExportReport {
            meta: ExportMeta {
                tool: "st-mem".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                repository: env!("CARGO_PKG_REPOSITORY").to_string(),
            },
            flash_used: analysis.usage.flash_used,
            flash_total: analysis.usage.flash_total,
            flash_percent: analysis.usage.flash_percent(),
            ram_used: analysis.usage.ram_used,
            ram_total: analysis.usage.ram_total,
            ram_percent: analysis.usage.ram_percent(),
            sections,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# st-mem Firmware Report\n\n");
        md.push_str(&format!("Generated by **{} v{}** — {}\n\n",
            self.meta.tool, self.meta.version, self.meta.repository));

        md.push_str("## Memory Usage\n\n");
        md.push_str("| Region | Used | Total | Percent |\n");
        md.push_str("|--------|------|-------|----------|\n");
        md.push_str(&format!("| FLASH | {} | {} | {:.1}% |\n",
            format_bytes(self.flash_used), format_bytes(self.flash_total), self.flash_percent));
        md.push_str(&format!("| RAM | {} | {} | {:.1}% |\n",
            format_bytes(self.ram_used), format_bytes(self.ram_total), self.ram_percent));

        if !self.sections.is_empty() {
            md.push_str("\n## Sections\n\n");
            md.push_str("| Name | Region | Address | Size | Flags |\n");
            md.push_str("|------|--------|---------|------|-------|\n");
            for sec in &self.sections {
                md.push_str(&format!("| {} | {} | {} | {} | {} |\n",
                    sec.name, sec.region, sec.address,
                    format_bytes(sec.size as u64),
                    sec.flags.join(", ")));
            }
        }

        md
    }
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
        assert!(report.contains("FIRMWARE SIZE"));
        assert!(report.contains("FLASH"));
        assert!(report.contains("RAM"));
    }

    #[test]
    fn test_export_json() {
        let analysis = crate::elf::analyze_elf_detailed("stm32dome",
            &crate::memory::MemoryConfig::from_file("memory.x").unwrap()
        ).unwrap();
        let report = ExportReport::from_analysis(&analysis);
        let json = report.to_json();
        assert!(json.contains("\"flash_used\""));
        assert!(json.contains("\"sections\""));
    }

    #[test]
    fn test_export_markdown() {
        let analysis = crate::elf::analyze_elf_detailed("stm32dome",
            &crate::memory::MemoryConfig::from_file("memory.x").unwrap()
        ).unwrap();
        let report = ExportReport::from_analysis(&analysis);
        let md = report.to_markdown();
        assert!(md.contains("# st-mem Firmware Report"));
        assert!(md.contains("## Memory Usage"));
    }
}
