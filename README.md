# st-mem

> GitHub: https://github.com/cx693/st-mem

嵌入式固件内存分析工具。解析 `memory.x` 链接脚本和 ELF 二进制文件，统计 FLASH / RAM 占用，以进度条方式直观展示。

```
  [FIRMWARE SIZE]
  +----------------------------------------------------------------+
  | FLASH [████░░░░░░░░░░░░░░░░░░░░░░░░░░]  15.1%    9 KB / 64 KB  |
  | RAM   [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0.0%     4 B / 20 KB  |
  +----------------------------------------------------------------+
```

## 前置依赖

### probe-rs

Runner 模式使用 [probe-rs](https://probe.rs/) 进行固件烧录，需要先安装：


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

## 安装

```bash
# 从本地源码安装
git clone https://github.com/cx693/st-mem.git
cd st-mem
cargo install --path .

# 发布后从 crates.io 安装
cargo install st-mem
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

### Runner 模式（分析 + 烧录）

Runner 模式通过 [probe-rs](https://probe.rs/) 进行固件烧录。`st-mem runner` 在调用 probe-rs 之前先分析固件内存占用，然后将所有参数透传给 `probe-rs run`。

```bash
st-mem runner [probe-rs 参数] <elf-path>
```

```bash
st-mem runner --chip STM32F103C8 --protocol swd target/thumbv7m-none-eabi/release/firmware
```

实际等效于：

```bash
# 1. st-mem 先分析固件大小并显示
# 2. 然后执行:
probe-rs run --chip STM32F103C8 --protocol swd target/thumbv7m-none-eabi/release/firmware
```

### 集成到 cargo

在项目的 `.cargo/config.toml` 中配置：

```toml
[target.thumbv7m-none-eabi]
runner = "st-mem runner --chip STM32F103C8 --protocol swd"
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

## 参数

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--memory-x <path>` | memory.x 文件路径 | `memory.x` |
| `--elf <path>` | ELF 文件路径（也可直接用位置参数） | - |
| `--width <n>` | 进度条宽度（字符数） | `30` |

## 库 API

在 Cargo.toml 中添加依赖：

```toml
[dependencies]
st-mem = { path = "st-mem" }
```

```rust
use st_mem::{MemoryConfig, analyze_elf, format_report};

let config = MemoryConfig::from_file("memory.x")?;
let usage = analyze_elf("firmware.elf", &config)?;
println!("{}", format_report(&usage, 30));
```

### API 说明

| 类型 / 函数 | 说明 |
|---|---|
| `MemoryConfig::from_file(path)` | 解析 memory.x 文件 |
| `MemoryConfig::parse(content)` | 从字符串解析 |
| `config.flash()` / `config.ram()` | 获取 FLASH / RAM 区域信息 |
| `analyze_elf(path, &config)` | 分析 ELF 文件，返回 `FirmwareUsage` |
| `format_report(&usage, width)` | 生成格式化的内存报告字符串 |
| `format_bytes(bytes)` | 字节数转可读格式（B / KB / MB） |
| `progress_bar(pct, width)` | 生成进度条字符串 |

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
# st-mem runner: 分析 FLASH/RAM 占用 → probe-rs 烧录
runner = "st-mem runner --chip STM32F103C8 --protocol swd"
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
