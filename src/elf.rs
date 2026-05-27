use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::memory::MemoryConfig;

/// Memory region type for a section.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RegionType {
    Flash,
    Ram,
    Other,
}

/// Information about a single ELF section.
#[derive(Debug, Clone, Serialize)]
pub struct SectionInfo {
    /// Section name from the ELF header.
    pub name: String,
    /// Virtual address where the section is loaded.
    pub address: u32,
    /// Section size in bytes.
    pub size: u32,
    /// Section flags (SHF_ALLOC, SHF_WRITE, SHF_EXECINSTR, etc.)
    pub flags: u32,
    /// Which memory region this section belongs to.
    pub region: RegionType,
}

impl SectionInfo {
    pub fn is_alloc(&self) -> bool {
        self.flags & 0x2 != 0
    }
    pub fn is_writable(&self) -> bool {
        self.flags & 0x1 != 0
    }
    pub fn is_executable(&self) -> bool {
        self.flags & 0x4 != 0
    }
}

/// Firmware memory usage analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct FirmwareUsage {
    /// FLASH bytes used by the firmware.
    pub flash_used: u64,
    /// RAM bytes used by the firmware.
    pub ram_used: u64,
    /// Total FLASH available (from memory.x).
    pub flash_total: u64,
    /// Total RAM available (from memory.x).
    pub ram_total: u64,
}

impl FirmwareUsage {
    /// FLASH usage as a percentage (0.0 - 100.0).
    pub fn flash_percent(&self) -> f64 {
        if self.flash_total == 0 {
            0.0
        } else {
            self.flash_used as f64 * 100.0 / self.flash_total as f64
        }
    }

    /// RAM usage as a percentage (0.0 - 100.0).
    pub fn ram_percent(&self) -> f64 {
        if self.ram_total == 0 {
            0.0
        } else {
            self.ram_used as f64 * 100.0 / self.ram_total as f64
        }
    }

    /// Remaining FLASH bytes.
    pub fn flash_free(&self) -> u64 {
        self.flash_total.saturating_sub(self.flash_used)
    }

    /// Remaining RAM bytes.
    pub fn ram_free(&self) -> u64 {
        self.ram_total.saturating_sub(self.ram_used)
    }
}

/// Parsed ELF analysis result containing usage and section breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct ElfAnalysis {
    /// Overall firmware usage.
    pub usage: FirmwareUsage,
    /// Per-section breakdown (allocated sections only).
    pub sections: Vec<SectionInfo>,
}

/// Analyze an ELF firmware binary and compute memory usage.
pub fn analyze_elf<P: AsRef<Path>>(elf_path: P, config: &MemoryConfig) -> Result<FirmwareUsage, String> {
    let path = elf_path.as_ref();
    let data = fs::read(path)
        .map_err(|e| format!("Failed to read ELF file {}: {}", path.display(), e))?;

    let analysis = analyze_elf_bytes(&data, config)?;
    Ok(analysis.usage)
}

/// Analyze an ELF firmware binary and return both usage and section breakdown.
pub fn analyze_elf_detailed<P: AsRef<Path>>(elf_path: P, config: &MemoryConfig) -> Result<ElfAnalysis, String> {
    let path = elf_path.as_ref();
    let data = fs::read(path)
        .map_err(|e| format!("Failed to read ELF file {}: {}", path.display(), e))?;

    analyze_elf_bytes(&data, config)
}

/// Analyze ELF binary from a byte slice.
pub fn analyze_elf_bytes(data: &[u8], config: &MemoryConfig) -> Result<ElfAnalysis, String> {
    if data.len() < 52 {
        return Err("File too small to be a valid 32-bit ELF".to_string());
    }

    // Validate ELF magic
    if &data[0..4] != b"\x7fELF" {
        return Err("Not a valid ELF file (bad magic)".to_string());
    }

    // ELF class: 4 = 32-bit, 8 = 64-bit
    let elf_class = data[4];
    if elf_class != 1 {
        return Err("Only 32-bit ELF files are supported".to_string());
    }

    let flash_region = config.flash();
    let ram_region = config.ram();

    let flash_origin = flash_region.map(|r| r.origin as u32).unwrap_or(0x0800_0000);
    let flash_length = flash_region.map(|r| r.length).unwrap_or(64 * 1024);
    let flash_end = flash_origin as u64 + flash_length;

    let ram_origin = ram_region.map(|r| r.origin as u32).unwrap_or(0x2000_0000);
    let ram_length = ram_region.map(|r| r.length).unwrap_or(20 * 1024);
    let ram_end = ram_origin as u64 + ram_length;

    // Parse ELF32 header
    let e_shoff = u32::from_le_bytes(data[0x20..0x24].try_into().unwrap()) as usize;
    let e_shentsize = u16::from_le_bytes(data[0x2E..0x30].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(data[0x30..0x32].try_into().unwrap()) as usize;
    let e_shstrndx = u16::from_le_bytes(data[0x32..0x34].try_into().unwrap()) as usize;

    if e_shentsize == 0 || e_shnum == 0 {
        return Ok(ElfAnalysis {
            usage: FirmwareUsage {
                flash_used: 0,
                ram_used: 0,
                flash_total: flash_length,
                ram_total: ram_length,
            },
            sections: Vec::new(),
        });
    }

    // Read section name string table
    let shstrtab = read_section_data(data, e_shoff, e_shstrndx, e_shentsize);

    let mut flash_used: u64 = 0;
    let mut ram_used: u64 = 0;
    let mut sections = Vec::new();

    for i in 0..e_shnum {
        let sh_off = e_shoff + i * e_shentsize;
        if sh_off + 40 > data.len() {
            break;
        }

        let sh_name_idx = u32::from_le_bytes(data[sh_off..sh_off + 4].try_into().unwrap()) as usize;
        let sh_type = u32::from_le_bytes(data[sh_off + 4..sh_off + 8].try_into().unwrap());
        let sh_flags = u32::from_le_bytes(data[sh_off + 8..sh_off + 12].try_into().unwrap());
        let sh_addr = u32::from_le_bytes(data[sh_off + 12..sh_off + 16].try_into().unwrap());
        let sh_size = u32::from_le_bytes(data[sh_off + 20..sh_off + 24].try_into().unwrap());

        // SHT_NULL = 0, skip null sections
        if sh_type == 0 {
            continue;
        }

        let name = read_string(&shstrtab, sh_name_idx);

        // SHF_ALLOC = 0x2: section occupies memory during process execution
        if sh_flags & 0x2 != 0 {
            let addr = sh_addr as u64;
            let region = if addr >= ram_origin as u64 && addr < ram_end {
                ram_used += sh_size as u64;
                RegionType::Ram
            } else if addr >= flash_origin as u64 && addr < flash_end {
                flash_used += sh_size as u64;
                RegionType::Flash
            } else {
                RegionType::Other
            };

            sections.push(SectionInfo {
                name,
                address: sh_addr,
                size: sh_size,
                flags: sh_flags,
                region,
            });
        }
    }

    // Sort sections: Flash first, then Ram, then Other; within each group by size descending
    sections.sort_by(|a, b| {
        let ra = match a.region { RegionType::Flash => 0, RegionType::Ram => 1, RegionType::Other => 2 };
        let rb = match b.region { RegionType::Flash => 0, RegionType::Ram => 1, RegionType::Other => 2 };
        ra.cmp(&rb).then_with(|| b.size.cmp(&a.size))
    });

    Ok(ElfAnalysis {
        usage: FirmwareUsage {
            flash_used,
            ram_used,
            flash_total: flash_length,
            ram_total: ram_length,
        },
        sections,
    })
}

/// Read raw section data bytes from the ELF file for a given section index.
fn read_section_data(data: &[u8], e_shoff: usize, index: usize, e_shentsize: usize) -> Vec<u8> {
    let sh_off = e_shoff + index * e_shentsize;
    if sh_off + 40 > data.len() {
        return Vec::new();
    }
    let sh_offset = u32::from_le_bytes(data[sh_off + 16..sh_off + 20].try_into().unwrap()) as usize;
    let sh_size = u32::from_le_bytes(data[sh_off + 20..sh_off + 24].try_into().unwrap()) as usize;
    if sh_offset + sh_size > data.len() {
        return Vec::new();
    }
    data[sh_offset..sh_offset + sh_size].to_vec()
}

/// Read a null-terminated string from a byte slice at the given offset.
fn read_string(strtab: &[u8], offset: usize) -> String {
    if offset >= strtab.len() {
        return format!("<unknown@{}>", offset);
    }
    let end = strtab[offset..]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(strtab.len() - offset);
    String::from_utf8_lossy(&strtab[offset..offset + end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryConfig;

    fn test_config() -> MemoryConfig {
        MemoryConfig::parse(
            "MEMORY {\n  FLASH : ORIGIN = 0x08000000, LENGTH = 64K\n  RAM : ORIGIN = 0x20000000, LENGTH = 20K\n}"
        ).unwrap()
    }

    #[test]
    fn test_invalid_data() {
        let config = test_config();
        assert!(analyze_elf_bytes(b"too small", &config).is_err());
        assert!(analyze_elf_bytes(b"NOT_ELF_DATA_HERE_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX", &config).is_err());
    }

    #[test]
    fn test_analyze_stm32dome() {
        let config = crate::memory::MemoryConfig::from_file("memory.x").unwrap();
        let analysis = analyze_elf_detailed("stm32dome", &config).unwrap();

        assert!(analysis.usage.flash_used > 0, "FLASH should have used bytes");
        assert!(analysis.usage.flash_total > 0);
        assert!(!analysis.sections.is_empty(), "Should have sections");

        // Verify sections are sorted: Flash before Ram
        let mut seen_ram = false;
        for sec in &analysis.sections {
            if sec.region == RegionType::Ram {
                seen_ram = true;
            }
            if seen_ram && sec.region == RegionType::Flash {
                panic!("Flash section found after Ram section - sort order broken");
            }
        }
    }
}
