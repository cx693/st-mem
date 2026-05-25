use std::fs;
use std::path::Path;

/// A single memory region (FLASH or RAM).
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub name: String,
    pub origin: u64,
    pub length: u64,
}

/// Parsed memory configuration from a linker script (memory.x).
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub regions: Vec<MemoryRegion>,
}

impl MemoryConfig {
    /// Parse a `memory.x` file at the given path.
    ///
    /// The file format follows GNU LD linker script syntax:
    /// ```text
    /// MEMORY
    /// {
    ///   FLASH : ORIGIN = 0x08000000, LENGTH = 64K
    ///   RAM : ORIGIN = 0x20000000, LENGTH = 20K
    /// }
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read {}: {}", path.as_ref().display(), e))?;
        Self::parse(&content)
    }

    /// Parse memory configuration from a string.
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut regions = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty()
                || line.starts_with("/*")
                || line.starts_with('*')
                || line.starts_with("//")
                || line.starts_with('#')
            {
                continue;
            }

            // Skip MEMORY { } wrapper lines
            if line.starts_with("MEMORY") || line == "{" || line == "}" {
                continue;
            }

            // Try to parse a region line:
            //   FLASH : ORIGIN = 0x08000000, LENGTH = 64K
            //   FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 64K
            if let Some(region) = parse_region_line(line) {
                regions.push(region);
            }
        }

        if regions.is_empty() {
            return Err("No memory regions found in memory.x".to_string());
        }

        Ok(MemoryConfig { regions })
    }

    /// Find a region by name (case-insensitive).
    pub fn find(&self, name: &str) -> Option<&MemoryRegion> {
        let name_lower = name.to_lowercase();
        self.regions.iter().find(|r| r.name.to_lowercase() == name_lower)
    }

    /// Get the FLASH region, if present.
    pub fn flash(&self) -> Option<&MemoryRegion> {
        self.find("FLASH")
    }

    /// Get the RAM region, if present.
    pub fn ram(&self) -> Option<&MemoryRegion> {
        self.find("RAM")
    }
}

fn parse_region_line(line: &str) -> Option<MemoryRegion> {
    // Format: NAME : ORIGIN = 0x..., LENGTH = ...
    // or:     NAME (flags) : ORIGIN = 0x..., LENGTH = ...
    let colon_pos = line.find(':')?;
    let name_part = line[..colon_pos].trim();

    // Extract name (strip attribute flags like (rx), (rwx), etc.)
    let name = if let Some(paren_pos) = name_part.find('(') {
        name_part[..paren_pos].trim().to_string()
    } else {
        name_part.to_string()
    };

    let rest = &line[colon_pos + 1..];

    // Parse ORIGIN
    let origin = parse_origin(rest)?;

    // Parse LENGTH
    let length = parse_length(rest)?;

    Some(MemoryRegion { name, origin, length })
}

fn parse_origin(s: &str) -> Option<u64> {
    let idx = s.find("ORIGIN")?;
    let after = s[idx + "ORIGIN".len()..].trim();
    let after = after.trim_start_matches('=').trim();
    parse_number(after.split(|c: char| c == ',').next()?.trim())
}

fn parse_length(s: &str) -> Option<u64> {
    let idx = s.find("LENGTH")?;
    let after = s[idx + "LENGTH".len()..].trim();
    let after = after.trim_start_matches('=').trim();
    let val = after.split(|c: char| c == '}').next()?.trim();
    let val = val.trim_end_matches(',').trim();
    parse_size(val)
}

fn parse_number(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).ok()
    } else if s.starts_with("0o") || s.starts_with("0O") {
        u64::from_str_radix(&s[2..], 8).ok()
    } else if s.starts_with("0b") || s.starts_with("0B") {
        u64::from_str_radix(&s[2..], 2).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Parse a size string like "64K", "20K", "1M", "512", etc.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    if s.ends_with('K') {
        let num: f64 = s[..s.len() - 1].trim().parse().ok()?;
        Some((num * 1024.0) as u64)
    } else if s.ends_with('M') {
        let num: f64 = s[..s.len() - 1].trim().parse().ok()?;
        Some((num * 1024.0 * 1024.0) as u64)
    } else {
        s.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let content = r#"
/* Linker script for the STM32F103C8T6 */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 64K
  RAM : ORIGIN = 0x20000000, LENGTH = 20K
}
"#;
        let config = MemoryConfig::parse(content).unwrap();
        assert_eq!(config.regions.len(), 2);

        let flash = config.flash().unwrap();
        assert_eq!(flash.origin, 0x08000000);
        assert_eq!(flash.length, 64 * 1024);

        let ram = config.ram().unwrap();
        assert_eq!(ram.origin, 0x20000000);
        assert_eq!(ram.length, 20 * 1024);
    }

    #[test]
    fn test_parse_with_flags() {
        let content = r#"
MEMORY
{
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 128K
  RAM (xrw) : ORIGIN = 0x20000000, LENGTH = 20K
}
"#;
        let config = MemoryConfig::parse(content).unwrap();
        let flash = config.flash().unwrap();
        assert_eq!(flash.length, 128 * 1024);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("64K"), Some(64 * 1024));
        assert_eq!(parse_size("20K"), Some(20 * 1024));
        assert_eq!(parse_size("1M"), Some(1024 * 1024));
        assert_eq!(parse_size("512"), Some(512));
        assert_eq!(parse_size("0.5K"), Some(512));
    }
}
