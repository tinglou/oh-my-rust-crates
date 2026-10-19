# macaddr-ouidb

[:us: English](README.md) | [:cn: 中文](README-cn.md)

高性能 MAC 地址 OUI（Organizationally Unique Identifier，组织唯一标识符）查询库，支持根据 MAC 地址查询厂商信息。

## 特性

- 🚀 **零成本抽象**：编译期生成查找表，运行时仅做二分查找
- 💾 **内存高效**：列式存储 + 紧凑编码
- ⚡ **快速查找**：O(log n) 时间复杂度，支持 24/28/36 位 OUI
- 🔍 **覆盖全面**：包含 40,000+ OUI-24 条目、6,600+ OUI-28（MA-M）条目、11,000+ OUI-36（MA-S/IAB）条目
- 🌍 **地区信息**：每个条目除组织名称外还携带 ISO 3166-1 alpha-2 国家/地区代码
- 🏢 **注册地址（可选）**：启用 `address` feature 后，每个条目还提供注册地址，可通过 `OuiEntry::address()` 获取
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
use macaddr_ouidb::{MacAddress, oui_db};
use std::str::FromStr;

// 创建 MAC 地址
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC").unwrap();

// 查询厂商信息（仅名称）
match mac.oui() {
    Some(org_name) => println!("厂商: {}", org_name),
    None => println!("未找到 OUI 信息"),
}

// 查询完整 OUI 条目（名称、地区、前缀）
let db = oui_db();
if let Some(entry) = db.lookup_mac(mac) {
    println!("厂商: {} ({})", entry.name(), entry.country_or_region_code());
    println!("OUI 前缀: {}", entry.prefix_hex());
}

// 判断是否为虚拟网卡
if mac.is_virtual_nic() {
    println!("这是虚拟网卡: {}", mac.oui().unwrap_or_else(|| "未知".to_string()));
}

// 判断地址类型
println!("单播: {}", mac.is_unicast());
println!("全局唯一: {}", mac.is_universal());
```

### 注册地址（可选）

启用 `address` feature 后，内置数据集会额外存储各组织的注册地址。
`OuiEntry::address()` / `OuiEntryOwned::address()` 返回该地址，而组织名称 `name()`
不包含地址。没有地址的条目（如 custom 条目，或从不含地址的文件加载的运行时数据库）
返回 `""`：

```rust,ignore
use macaddr_ouidb::oui_db;

let db = oui_db();
let mac = "00:00:01:00:00:00".parse().unwrap();
if let Some(entry) = db.lookup_mac(mac) {
    println!("厂商: {}", entry.name());
    println!("地址: {}", entry.address());
}
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

### 全局数据库与运行时加载

全局入口 `oui_db()` 默认使用编译期内置数据库。启用 `std` feature 后，可在运行时
从数据文件加载数据库（`OuiDbOwned`）并安装到全局。`set_oui_db` 可多次调用，
最新一次生效。全局基于 `arc-swap` 实现：并发持有中的句柄（`OuiDbRef`）与查询结果
（`OuiEntry`）会各自保活其来源数据库，被替换的旧实例在最后一个引用释放后自动回收
（不再泄漏）。`reset_oui_db` 可恢复为内置数据库：

```rust,no_run
use macaddr_ouidb::{set_oui_db, reset_oui_db, oui_db, OuiDb, OuiDbOwned, OUI_DB_BUNDLED};

// 从两个文件加载并安装运行时数据库。
// `custom-mac-oui.txt` 中的条目会合并覆盖 `ieee-mac-oui.txt`。
// ieee 文件必须包含 `# Updated:` 头行。
let owned = OuiDbOwned::from_files("custom-mac-oui.txt", "ieee-mac-oui.txt").unwrap();
set_oui_db(owned.into());

// 此后所有查询都路由到已安装的数据库。
assert!(matches!(&*oui_db(), OuiDb::Owned(_)));

// 可再次替换（例如热更新）；最新一次生效。
set_oui_db(OuiDb::Bundled(OUI_DB_BUNDLED));
assert!(matches!(&*oui_db(), OuiDb::Bundled(_)));

// 重置回内置数据库。
reset_oui_db();
assert!(matches!(&*oui_db(), OuiDb::Bundled(_)));
```

借用式查询（`lookup` / `lookup_mac`）返回 `OuiEntry<'_>`，仅在数据库借用期内有效，
适合即取即用（零拷贝）。当条目需要比句柄活得更久（例如存入结构体、由函数返回，
或在 `set_oui_db` 替换数据库后继续使用）时，使用拥有式查询
`lookup_owned` / `lookup_mac_owned`（或 [`MacAddress::oui_entry`]）；它们返回
`OuiEntryOwned`，自身持有数据来源，并实现 `Clone`、`Debug` 与 `Display`。

未启用 `std` feature 时，仅提供内置数据库，`set_oui_db` / `OuiDbOwned` 不参与编译。

## API 文档

### `MacAddress`

以太网 MAC 地址类型（6 字节）。

```rust
use macaddr_ouidb::{MacAddress, oui_db};
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
let oui = mac.oui();                 // Option<String>  （仅名称）
let db = oui_db();
let entry = db.lookup_mac(mac);        // Option<OuiEntry<'_>>   （借用式条目）
let entry = db.lookup_mac_owned(mac);  // Option<OuiEntryOwned>  （拥有式条目）
let entry = mac.oui_entry();           // Option<OuiEntryOwned>  （拥有式条目）
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
| `address`  | 包含组织注册地址                      | 否       |

## no_std 支持

本 crate 支持 `no_std` 环境。在 `no_std` 环境中使用时，请禁用默认 features：

```toml
[dependencies]
macaddr-ouidb = { version = "0.2", default-features = false }
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

启用 `address` feature 后，当组织存在注册地址时会额外打印一行 `Address:`。

## 性能

- **查找耗时**：O(log n) 二分查找
- **内存占用**：约 1.0 MB 内嵌静态数据（release 构建实测），覆盖 40,000+ OUI-24、6,600+ OUI-28、11,000+ OUI-36 条目
- **初始化**：零运行时初始化（编译期生成）
