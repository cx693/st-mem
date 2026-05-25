use std::fs;
use std::path::Path;

use crate::memory::MemoryConfig;

/// Firmware memory usage analysis result.
#[derive(Debug, Clone)]
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

/// Analyze an ELF firmware binary and compute memory usage.
///
/// Reads the ELF section headers and classifies allocated sections
/// into FLASH or RAM based on their virtual address ranges from `memory.x`.
///
/// # Arguments
///
/// * `elf_path` - Path to the ELF binary.
/// * `config` - Parsed memory configuration from `memory.x`.
///
/// # Returns
///
/// A `FirmwareUsage` with FLASH/RAM used and total sizes.
pub fn analyze_elf<P: AsRef<Path>>(elf_path: P, config: &MemoryConfig) -> Result<FirmwareUsage, String> {
    let path = elf_path.as_ref();
    let data = fs::read(path)
        .map_err(|e| format!("Failed to read ELF file {}: {}", path.display(), e))?;

    analyze_elf_bytes(&data, config)
}

/// Analyze ELF binary from a byte slice.
pub fn analyze_elf_bytes(data: &[u8], config: &MemoryConfig) -> Result<FirmwareUsage, String> {
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

    if e_shentsize == 0 || e_shnum == 0 {
        return Ok(FirmwareUsage {
            flash_used: 0,
            ram_used: 0,
            flash_total: flash_length,
            ram_total: ram_length,
        });
    }

    let mut flash_used: u64 = 0;
    let mut ram_used: u64 = 0;

    for i in 0..e_shnum {
        let sh_off = e_shoff + i * e_shentsize;
        if sh_off + 24 > data.len() {
            break;
        }

        let sh_flags = u32::from_le_bytes(data[sh_off + 8..sh_off + 12].try_into().unwrap());
        let sh_addr = u32::from_le_bytes(data[sh_off + 12..sh_off + 16].try_into().unwrap());
        let sh_size = u32::from_le_bytes(data[sh_off + 20..sh_off + 24].try_into().unwrap());

        // SHF_ALLOC = 0x2: section occupies memory during process execution
        if sh_flags & 0x2 != 0 {
            let addr = sh_addr as u64;
            if addr >= ram_origin as u64 && addr < ram_end {
                ram_used += sh_size as u64;
            } else if addr >= flash_origin as u64 && addr < flash_end {
                flash_used += sh_size as u64;
            }
        }
    }

    Ok(FirmwareUsage {
        flash_used,
        ram_used,
        flash_total: flash_length,
        ram_total: ram_length,
    })
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
}
