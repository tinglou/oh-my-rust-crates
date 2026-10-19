# macaddr-ouidb

[:us: English](README.md) | [:cn: 中文](README-cn.md)

高性能 MAC 地址 OUI（Organizationally Unique Identifier，组织唯一标识符）查询库，支持根据 MAC 地址查询厂商信息。

## 特性

- 🚀 **零成本抽象**：编译期生成查找表，运行时仅做二分查找
- 💾 **内存高效**：列式存储 + 紧凑编码
- ⚡ **快速查找**：O(log n) 时间复杂度，支持 24/28/36 位 OUI
- 🔍 **覆盖全面**：包含 40,000+ OUI-24 条目、6,600+ OUI-28（MA-M）条目、11,000+ OUI-36（MA-S/IAB）条目
- 🌍 **地区信息**：每个条目除组织名称外还携带 ISO 3166-1 alpha-2 国家/地区代码
- 🎯 **虚拟网卡识别**：内置常见虚拟化平台网卡识别（VMware、QEMU、VirtualBox、OpenStack 等）
- 📦 **灵活集成**：支持 `serde` 序列化，可选 `pnet` 互操作
- 🔒 **no_std 支持**：可在 `no_std` 环境下使用（std 相关功能可选）
- 🔢 **多种 MAC 地址格式**：支持解析多种格式的 MAC 地址：
  - 冒号分隔：`00:11:22:33:44:55`
  - 连字符分隔：`00-11-22-33-44-55`
  - Cisco IOS 格式：`001A.2B3C.4D5E`
  - 连续十六进制：`001A2B3C4D5E`

## 快速开始

### 基本用法

```rust
use macaddr_ouidb::MacAddress;
use std::str::FromStr;

// 创建 MAC 地址
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC").unwrap();

// 查询厂商信息（仅名称）
match mac.oui() {
    Some(org_name) => println!("厂商: {}", org_name),
    None => println!("未找到 OUI 信息"),
}

// 查询完整 OUI 条目（名称、地区、前缀）
if let Some(entry) = mac.oui_entry() {
    println!("厂商: {} ({})", entry.name(), entry.country_or_region_code());
    println!("OUI 前缀: {}", entry.prefix_hex());
}

// 判断是否为虚拟网卡
if mac.is_virtual_nic() {
    println!("这是虚拟网卡: {}", mac.oui().unwrap_or("未知"));
}

// 判断地址类型
println!("单播: {}", mac.is_unicast());
println!("全局唯一: {}", mac.is_universal());
```

### Serde 支持

启用 `serde` feature 后，支持 JSON 序列化：

```rust,ignore
use macaddr_ouidb::MacAddress;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Device {
    mac: MacAddress,
    name: String,
}
```

## API 文档

### `MacAddress`

以太网 MAC 地址类型（6 字节）。

```rust
use macaddr_ouidb::MacAddress;
use std::str::FromStr;

// 构造函数
let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
let mac = MacAddress::new6(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
let mac: MacAddress = "00-11-22-33-44-55".parse().unwrap();
let bytes = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
let mac: MacAddress = "001A2B3C4D5E".parse().unwrap(); // 连续十六进制
let zero = MacAddress::zero();
let broadcast = MacAddress::broadcast();

// 属性检查
let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
assert!(mac.is_unicast());
assert!(!mac.is_multicast());
assert!(!mac.is_broadcast());
assert!(mac.is_universal());
assert!(!mac.is_local());
assert!(!mac.is_zero());

// 方法
let octets = mac.octets();           // [u8; 6]
let value = mac.to_u64();            // u64
let oui = mac.oui();                 // Option<&'static str> （仅名称）
let entry = mac.oui_entry();         // Option<OuiEntry<'static>> （条目）
let is_virtual = mac.is_virtual_nic(); // bool
```

## 数据格式

支持三种 OUI 类型：

| 类型          | 位数 | 格式            | 说明                                                     |
| ------------- | ---- | --------------- | -------------------------------------------------------- |
| OUI-24 (MA-L) | 24   | `XX:XX:XX`      | 传统分配，前 3 字节                                       |
| OUI-28 (MA-M) | 28   | `XX:XX:XX:X`    | 中型分配，前 3 字节 + 第 4 字节高 4 位                     |
| OUI-36 (MA-S) | 36   | `XX:XX:XX:XX:X` | 小型分配，前 4 字节 + 第 5 字节高 4 位                     |

查找时自动匹配最精确的类型。

## 数据来源

OUI 数据来自 IEEE 官方注册机构（MA-L、MA-M、MA-S/IAB），由 [`ouidb-builder`](../ouidb-builder) 工具在编译期生成 `ieee-mac-oui.txt`。一份手工维护的 `custom-mac-oui.txt` 补充了 IEEE 数据中缺少的虚拟网卡前缀（QEMU、Bochs、OpenStack 等）。

`build.rs` 在编译期合并两个数据源：

- 组织名称存储为 `<2 位地区代码><名称>`（例如 `JPSony`）；国家/地区代码从 IEEE 组织地址中提取。
- IEEE 厂商提交数据中包含的不可见 Unicode 字符（不间断空格、零宽空格、软连字符等）在生成时已清理。
- 因 IEEE 重新分配导致的重复前缀会去重，保留**最新**的分配（详见 `ouidb-builder/duplicate-prefix.log`）。
- 若某前缀同时存在于 `custom-mac-oui.txt` 和 IEEE 数据中，构建会**报错退出**，而非静默覆盖。

数据通过 `build.rs` 在编译期自动生成，确保：
- 与源码版本一致
- 无需运行时加载
- 零初始化开销

## Features

| Feature    | 说明                                 | 默认启用 |
| ---------- | ----------------------------------- | -------- |
| `std`      | 启用 std 库支持                      | 是       |
| `serde`    | 启用 serde 序列化                    | 是       |
| `pnet`     | 启用 pnet 互操作                     | 是       |
| `fullname` | 使用完整（未截断）组织名称数据集      | 否       |

## no_std 支持

本 crate 支持 `no_std` 环境。在 `no_std` 环境中使用时，请禁用默认 features：

```toml
[dependencies]
macaddr-ouidb = { version = "0.1", default-features = false }
```

## CLI 工具

本 crate 包含一个 CLI 工具 `mac-oui`，用于查询 MAC 地址信息：

```console
$ cargo run --bin mac-oui -- 00:55:DA:0A:BB:CC
MAC Address: 00:55:da:0a:bb:cc
Formatted:   00:55:da:0a:bb:cc
Octets:      00-55-DA-0A-BB-CC

Address Type:
  Unicast:     Yes
  Multicast:   No
  Broadcast:   No
  Universal:   Yes
  Local:       No

Organization: Shinko Technos (JP)
```

## 性能

- **查找耗时**：O(log n) 二分查找
- **内存占用**：约 1.0 MB 内嵌静态数据（release 构建实测），覆盖 40,000+ OUI-24、6,600+ OUI-28、11,000+ OUI-36 条目
- **初始化**：零运行时初始化（编译期生成）
