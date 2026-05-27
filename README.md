# st-mem

[![Crates.io](https://img.shields.io/crates/v/st-mem)](https://crates.io/crates/st-mem)
[![License](https://img.shields.io/crates/l/st-mem)](https://github.com/cx693/st-mem)
[![Docs.rs](https://docs.rs/st-mem/badge.svg)](https://docs.rs/st-mem)

嵌入式固件内存分析工具。解析 `memory.x` 链接脚本和 ELF 二进制文件，统计 FLASH / RAM 占用，以进度条方式直观展示。

```
  [FIRMWARE SIZE]
  +----------------------------------------------------------------+
  | FLASH [████░░░░░░░░░░░░░░░░░░░░░░░░░░]  15.1%    9 KB / 64 KB  |
  | RAM   [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0.0%     4 B / 20 KB  |
  +----------------------------------------------------------------+
```

## 功能

- 解析 GNU LD `memory.x` 链接脚本，提取 FLASH / RAM 区域定义
- 解析 ELF 二进制文件的 section headers，按地址范围归类到 FLASH / RAM
- 以进度条和百分比直观展示固件内存占用
- **Section 拆分统计**：单独展示每个 section 的占用、占比和进度条（`--sections`）
- **数据导出**：输出 JSON / Markdown 格式报告，适用于日志归档和 CI 流水线（`--export`）
- **版本信息**：`-V` 显示版本、作者和项目地址
- Runner 模式：分析完成后自动调用 [probe-rs](https://probe.rs/) 烧录固件
- 跨平台支持（macOS / Linux / Windows）

## 安装

### 从 crates.io 安装（推荐）

```bash
cargo install st-mem
```

### 从源码安装

```bash
git clone https://github.com/cx693/st-mem.git
cd st-mem
cargo install --path .
```

安装后获得 `st-mem` 命令。

## 使用

### 分析固件内存

```bash
st-mem <elf-path>
```

```bash
st-mem target/thumbv7m-none-eabi/release/firmware
```

指定 `memory.x` 路径：

```bash
st-mem target/thumbv7m-none-eabi/release/firmware --memory-x path/to/memory.x
```

### Section 拆分统计

使用 `--sections` 查看每个 section 的独立占用：

```bash
st-mem firmware.elf --sections
```

```
  [FIRMWARE SIZE]
  +----------------------------------------------------------------+
  | FLASH [████░░░░░░░░░░░░░░░░░░░░░░░░░░]  15.1%    9 KB / 64 KB  |
  | RAM   [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0.0%     4 B / 20 KB  |
  +----------------------------------------------------------------+

  [FLASH Sections] (9 KB total)
  NAME               SIZE       %  BAR
  .text              7 KB   11.8%  [███░░░░░░░░░░░░░░░░░░░░░░░░░░░]
  .rodata            1 KB    2.9%  [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]
  .vector_table     304 B    0.5%  [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]

  [RAM Sections] (4 B total)
  NAME               SIZE       %  BAR
  .bss                4 B    0.0%  [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]
```

### 数据导出

导出 JSON 格式（适用于程序解析和 CI 流水线）：

```bash
st-mem firmware.elf --export json > report.json
```

导出 Markdown 格式（适用于文档归档）：

```bash
st-mem firmware.elf --export md > report.md
```

### 查看版本

```bash
st-mem -V
```

```
st-mem v0.1.2
Parse embedded firmware memory layout from memory.x linker scripts and ELF binaries, display FLASH/RAM usage with progress bars
Author: cx693
License: MIT
Repository: https://github.com/cx693/st-mem
```

### Runner 模式（分析 + 烧录）

Runner 模式通过 [probe-rs](https://probe.rs/) 进行固件烧录。`st-mem runner` 在调用 probe-rs 之前先分析固件内存占用，然后将所有参数透传给 `probe-rs run`。

格式：st-mem 自身参数放在 `runner` **之前**，probe-rs 参数放在 `runner` **之后**。

```bash
st-mem [st-mem 参数] runner [probe-rs 参数] <elf-path>
```

```bash
st-mem runner --chip STM32F103C8 --protocol swd target/thumbv7m-none-eabi/release/firmware
```

带 st-mem 参数的用法：

```bash
st-mem --sections --width 40 runner --chip STM32F103C8 --protocol swd firmware.elf
```

实际等效于：

```bash
# 1. st-mem 先分析固件大小并显示（带 --sections 和 --width 40）
# 2. 然后执行:
probe-rs run --chip STM32F103C8 --protocol swd firmware.elf
```

### 前置依赖：probe-rs

Runner 模式需要先安装 [probe-rs](https://probe.rs/)：

```bash
cargo install probe-rs-tools
```

probe-rs 支持的下载器与芯片：

| 下载器 | 协议参数 |
|---|---|
| ST-Link | `--protocol swd` |
| J-Link | `--protocol jtag` / `--protocol swd` |
| CMSIS-DAP | `--protocol swd` |

芯片通过 `--chip` 参数指定，例如：

```bash
--chip STM32F103C8
--chip STM32F407VG
--chip nRF52840
```

完整芯片列表见 [probe-rs 目标支持](https://probe.rs/targets/)。

### 集成到 cargo

在项目的 `.cargo/config.toml` 中配置：

```toml
[target.thumbv7m-none-eabi]
runner = "st-mem runner --chip STM32F103C8 --protocol swd"
```

st-mem 参数放在 `runner` 之前：

```toml
[target.thumbv7m-none-eabi]
runner = "st-mem --sections --width 40 runner --chip STM32F103C8 --protocol swd"
```

配置后 `cargo run` 会自动先显示固件大小再烧录：

```
  [FIRMWARE SIZE]
  +----------------------------------------------------------------+
  | FLASH [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   1.0%   688 B / 64 KB  |
  | RAM   [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0.0%     4 B / 20 KB  |
  +----------------------------------------------------------------+

[INFO] Firmware: target/thumbv7m-none-eabi/release/stm32dome
[FLASH] Programming via probe-rs...
      Erasing ✔ 100%
  Programming ✔ 100%
      Finished in 0.86s
```

如果只需编译后查看大小，可在 `.cargo/config.toml` 添加 alias：

```toml
[alias]
r = "run --release"
```

```bash
cargo r   # 编译 + 分析 + 烧录
```

## 命令行参数

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--memory-x <path>` | memory.x 文件路径 | `memory.x` |
| `--elf <path>` | ELF 文件路径（也可直接用位置参数） | - |
| `--width <n>` | 进度条宽度（字符数） | `30` |
| `--sections` | 显示每个 section 的独立占用（默认关闭） | - |
| `--export <fmt>` | 导出报告：`json` 或 `md`（默认不导出） | - |
| `--version`, `-V` | 显示版本、作者和项目地址 | - |
| `--help`, `-h` | 显示帮助信息 | - |

## 库 API

`st-mem` 同时提供库 API，可在其他 Rust 项目中直接调用。

在 Cargo.toml 中添加依赖：

```toml
[dependencies]
st-mem = "0.1"
```

```rust
use st_mem::{MemoryConfig, analyze_elf, analyze_elf_detailed, format_report, format_sections, ExportReport};

let config = MemoryConfig::from_file("memory.x")?;
let usage = analyze_elf("firmware.elf", &config)?;
println!("{}", format_report(&usage, 30));

// 带 section 详情
let analysis = analyze_elf_detailed("firmware.elf", &config)?;
println!("{}", format_sections(&analysis, 30));

// 导出 JSON
let report = ExportReport::from_analysis(&analysis);
println!("{}", report.to_json());
```

### API 说明

| 类型 / 函数 | 说明 |
|---|---|
| `MemoryConfig::from_file(path)` | 解析 memory.x 文件 |
| `MemoryConfig::parse(content)` | 从字符串解析 |
| `config.flash()` / `config.ram()` | 获取 FLASH / RAM 区域信息 |
| `analyze_elf(path, &config)` | 分析 ELF 文件，返回 `FirmwareUsage` |
| `analyze_elf_detailed(path, &config)` | 分析 ELF 文件，返回 `ElfAnalysis`（含 section 详情） |
| `format_report(&usage, width)` | 生成格式化的内存报告字符串 |
| `format_sections(&analysis, width)` | 生成 section 拆分统计表格 |
| `ExportReport::from_analysis(&analysis)` | 构建可导出的报告结构 |
| `report.to_json()` | 导出 JSON 格式字符串 |
| `report.to_markdown()` | 导出 Markdown 格式字符串 |
| `format_bytes(bytes)` | 字节数转可读格式（B / KB / MB） |
| `progress_bar(pct, width)` | 生成进度条字符串 |
| `print_report(elf_path, memory_x_path)` | 分析并打印报告到 stdout |

## memory.x 格式

工具解析标准 GNU LD 链接脚本格式：

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 64K
  RAM : ORIGIN = 0x20000000, LENGTH = 20K
}
```

支持带属性标志的写法：

```ld
MEMORY
{
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 128K
  RAM (xrw) : ORIGIN = 0x20000000, LENGTH = 20K
}
```

支持的长度单位：`K`（KB）、`M`（MB），也支持小数如 `0.5K`。

## 跨平台

`st-mem` 本身是纯 Rust 二进制，不依赖平台特定工具。Runner 模式依赖的 [probe-rs](https://probe.rs/) 同样跨平台：

| 平台 | st-mem | probe-rs |
|---|---|---|
| macOS (aarch64 / x86_64) | 支持 | 支持 |
| Linux (x86_64 / aarch64) | 支持 | 支持 |
| Windows (x86_64) | 支持 | 支持 |

Runner 配置无需区分平台，直接使用 `st-mem runner` 即可，所有平台配置一致。

## 配置模版

`.cargo/config.toml` 完整示例：

```toml
[target.thumbv7m-none-eabi]
# ============================================================
# Runner — st-mem runner (跨平台，先分析内存再烧录)
# ============================================================
# st-mem 自身参数放 runner 前面，probe-rs 参数放 runner 后面
# st-mem [--sections] [--width N] [--memory-x PATH] runner [--chip X] [--protocol Y]
# ============================================================
# 基础用法（仅分析+烧录，不显示 section 详情）:
runner = "st-mem runner --chip STM32F103C8 --protocol swd"
# 带 section 详情和自定义宽度:
# runner = "st-mem --sections --width 40 runner --chip STM32F103C8 --protocol swd"
# ============================================================
# 不使用内存分析时，直接用 probe-rs:
# runner = "probe-rs run --chip STM32F103C8 --protocol swd"
# ============================================================
rustflags = [
  "-C", "link-arg=-Tlink.x",
#   "-C", "link-arg=-Tdefmt.x",
]

[build]
target = "thumbv7m-none-eabi"

[env]
DEFMT_LOG = "info"

[alias]
r = "run --release"
```

## License

MIT
