# ouidb-builder

下载并转换 IEEE OUI（组织唯一标识符）数据的构建工具。

## 功能特性

- **下载** — 从 standards-oui.ieee.org 拉取最新 IEEE OUI CSV 数据（MA-L、MA-M、MA-S 三类）
- **转换** — 将三个 CSV 合并为单一精简文本文件，附国家地区代码
- **后缀裁剪** — 自动去除公司名称末尾的常见法律形式后缀（Inc.、Ltd.、GmbH、有限公司等）
- **国家代码提取** — 从组织地址中解析 2 字母国家代码

## 快速开始

```bash
# 下载 IEEE OUI 数据文件
cargo run -p ouidb-builder -- download

# 转换为精简格式（去除公司后缀）
cargo run -p ouidb-builder -- convert

# 转换时保留公司全名
cargo run -p ouidb-builder -- convert --keep-suffix
```

## 命令说明

### `download`

下载三个 IEEE OUI CSV 文件到 `db/` 目录：

| 文件 | 来源 | 大小 |
|---|---|---|
| `db/oui.csv` | MA-L（24位） | ~3.8 MB |
| `db/iab.csv` | MA-M（28位） | ~380 KB |
| `db/oui36.csv` | MA-S（36位） | ~670 KB |

同时写入 `db/timestamp.txt`，内容为当前日期（`YYYY-MM-DD`）。

### `convert`

解析三个 CSV 文件，输出统一格式文件。

**默认输出**（`ieee-mac-oui.txt`）：去除公司后缀，约 5.2 万条

```
# Organizationally Unique Identifiers (OUIs)
# Original data comes from:
# MA-L(24 bits): https://standards-oui.ieee.org/oui/oui.csv
# MA-M(28 bits): https://standards-oui.ieee.org/iab/iab.csv
# MA-S(36 bits): https://standards-oui.ieee.org/oui36/oui36.csv
# Updated: 2026-10-16
286FB9 CN Nokia Shanghai Bell
E80AB9 US Cisco Systems
F0EE7A US Apple
```

**使用 `--keep-suffix`**（`ieee-mac-oui-fullname.txt`）：保留完整公司名

```
286FB9 CN Nokia Shanghai Bell Co., Ltd.
E80AB9 US Cisco Systems, Inc
F0EE7A US Apple, Inc.
```

## 输出格式

每行：`<MAC前缀> <2字母国家代码> <组织名称>`

- **MAC 前缀**：大写十六进制，6/7/9 位分别对应 24/28/36 位 OUI
- **国家代码**：从地址中提取的 2 字母 ISO 代码（`US`、`CN`、`DE` 等），无法识别时显示 `--`
- **组织名称**：默认去除公司后缀，传入 `--keep-suffix` 则保留原始名称

## 公司后缀裁剪

默认情况下，`convert` 会从组织名称末尾去除常见的企业法律形式缩写。支持的后缀包括：

| 地区 | 后缀 |
|---|---|
| 美国 | Inc.、Corp.、Corporation、LLC、LLP、LP、PC、Incorporated |
| 英国 | Ltd.、Limited、PLC |
| 德国 | GmbH、AG、KG、GmbH & Co. KG |
| 中国 | Co., Ltd.、Corp., Ltd. |
| 日本 | K.K.、G.K. |
| 新加坡 | Pte. Ltd. |
| 澳大利亚 | Pty. Ltd. |
| 荷兰 | BV、NV |
| 法国/西班牙 | S.A.、S.A.S.、S.A.U. |
| 意大利 | S.p.A.、S.r.l. |
| 北欧 | A/S、AB、Oy、ApS |
| 其他 | ULC、s.r.o.、Sdn Bhd |

大小写不敏感匹配，支持 `Co.,LTD`、`co., ltd.`、`CO., LTD.` 等各种变体。

**统计输出示例**：
```
Done: 52145 entries written to ../macaddr-ouidb/ieee-mac-oui.txt
Trimmed: 38259/52145 (73.4%), unique: 20242/28138 (71.9%), saved: 136793 bytes
```

## 集成方式

输出文件由 [`macaddr-ouidb`](../macaddr-ouidb) 在其 `build.rs` 编译期读取使用。

## 数据来源

所有数据来自 [IEEE 标准注册机构](https://standards-oui.ieee.org/)。

## 许可证

Apache-2.0