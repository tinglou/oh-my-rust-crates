//! OUI 数据文件解析（`std`）

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// 中间数据结构：单个 OUI 条目
#[derive(Debug, Clone)]
pub struct OuiEntry {
    /// 原始十六进制前缀字符串（如 "000001", "0001A2B" 等）
    pub prefix_str: String,
    /// 组织名称（已拼接为 `CC + ORG`）
    pub org_name: String,
    /// 前缀位长度（24/28/36）
    pub bit_len: usize,
}

/// 解析 `PREFIX CC ORG` 格式文件（ieee-mac-oui.txt / ieee-mac-oui-fullname.txt /
/// ieee-mac-oui-address.txt / custom-mac-oui.txt）
///
/// 文件不存在时返回空列表。
///
/// - `PREFIX`：6/7/9 字符 hex 前缀（24/28/36 位）
/// - `CC`：2 字节国家码
/// - `ORG`：组织名称（可能含空格）；可能带有以 `|` 分隔的地址后缀
/// - `keep_address`：为 `true` 时保留 `ORG` 中 `|` 之后的地址部分，
///   为 `false` 时在第一个 `|` 处截断并丢弃其后内容
///
/// 返回的 [`OuiEntry::org_name`] 已拼接为 `CC + ORG`，
/// 后续 `names` blob 中每条目即为 cc(2 字节) + name，与 `oui.rs` 的解码逻辑对应。
pub fn parse_oui_file(path: &Path, keep_address: bool) -> Vec<OuiEntry> {
    if !path.exists() {
        return Vec::new();
    }
    let file =
        fs::File::open(path).unwrap_or_else(|e| panic!("Failed to open {}: {e}", path.display()));
    parse_oui_reader(BufReader::new(file), keep_address)
}

/// 从任意 [`BufRead`] 源解析 OUI 条目（便于测试）
pub fn parse_oui_reader<R: BufRead>(reader: R, keep_address: bool) -> Vec<OuiEntry> {
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let line = line.trim();

        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 格式：PREFIX CC ORG[|ADDRESS]
        let mut parts = line.splitn(3, ' ');
        let prefix_str = parts.next().unwrap().trim();
        let cc = parts.next().unwrap_or("").trim();
        let org_name = parts.next().unwrap_or("").trim();

        // 根据前缀长度判断位数
        let bit_len = match prefix_str.len() {
            6 => 24,
            7 => 28,
            9 => 36,
            _ => {
                panic!(
                    "Invalid prefix length {} for '{}': expected 6, 7, or 9 characters (24, 28, or 36 bits)",
                    prefix_str.len(),
                    prefix_str
                );
            }
        };

        // `ORG` 可能带有以 `|` 分隔的地址后缀；按 `keep_address` 保留或截断
        let org_name = if keep_address {
            org_name
        } else {
            org_name.split_once('|').map_or(org_name, |(name, _)| name)
        };

        // 拼接新 name：CC + ORG
        let combined = format!("{}{}", cc, org_name);

        entries.push(OuiEntry {
            prefix_str: prefix_str.to_uppercase(),
            org_name: combined,
            bit_len,
        });
    }

    entries
}

/// 从任意 [`BufRead`] 源解析 `# Updated:` 时间戳（便于测试）
pub fn parse_updated_reader<R: BufRead>(reader: R, src: &str) -> String {
    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let line = line.trim();
        // 允许头部注释块中出现空行
        if line.is_empty() {
            continue;
        }
        if !line.starts_with('#') {
            break; // 注释头结束
        }
        if let Some(rest) = line.strip_prefix("# Updated:") {
            return rest.trim().to_string();
        }
    }
    panic!("Missing '# Updated:' comment in {src}");
}

/// 解析十六进制前缀为字节（奇数长度字符补 0）
pub fn parse_prefix_hex(prefix_str: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(prefix_str.len().div_ceil(2));
    let mut i = 0;
    while i < prefix_str.len() {
        let end = (i + 2).min(prefix_str.len());
        let hex_part = &prefix_str[i..end];
        // 如果只有一个字符，补 0
        let hex_part = if hex_part.len() == 1 {
            format!("{}0", hex_part)
        } else {
            hex_part.to_string()
        };
        bytes.push(u8::from_str_radix(&hex_part, 16).unwrap_or(0));
        i += 2;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use std::io::Cursor;

    #[test]
    fn parse_oui_lines() {
        let data = "\
# comment line
000001 US Some Company Inc.

00000A US Another Org
0001A2B US Odd Length
0001A2BCD US 36-bit Org
";
        let entries = parse_oui_reader(Cursor::new(data), false);
        assert_eq!(entries.len(), 4);

        assert_eq!(entries[0].prefix_str, "000001");
        assert_eq!(entries[0].bit_len, 24);
        assert_eq!(entries[0].org_name, "USSome Company Inc.");

        assert_eq!(entries[1].prefix_str, "00000A");
        assert_eq!(entries[1].org_name, "USAnother Org");

        assert_eq!(entries[2].prefix_str, "0001A2B");
        assert_eq!(entries[2].bit_len, 28);

        assert_eq!(entries[3].prefix_str, "0001A2BCD");
        assert_eq!(entries[3].bit_len, 36);
    }

    #[test]
    fn parse_address_suffix_is_kept_or_dropped() {
        let data = "000001 US Some Company|1 Main St, Springfield US 12345\n";

        // keep_address = true: the `|`-separated suffix is retained
        let kept = parse_oui_reader(Cursor::new(data), true);
        assert_eq!(
            kept[0].org_name,
            "USSome Company|1 Main St, Springfield US 12345"
        );

        // keep_address = false: everything from the first `|` on is dropped
        let dropped = parse_oui_reader(Cursor::new(data), false);
        assert_eq!(dropped[0].org_name, "USSome Company");

        // A line without an address is unchanged either way
        let plain = "000002 US Plain Org\n";
        assert_eq!(
            parse_oui_reader(Cursor::new(data), false)[0].org_name,
            "USSome Company"
        );
        assert_eq!(
            parse_oui_reader(Cursor::new(plain), false)[0].org_name,
            "USPlain Org"
        );
        assert_eq!(
            parse_oui_reader(Cursor::new(plain), true)[0].org_name,
            "USPlain Org"
        );
    }

    #[test]
    #[should_panic(expected = "Invalid prefix length")]
    fn parse_invalid_prefix_length_panics() {
        let data = "0000 US Too Short\n";
        parse_oui_reader(Cursor::new(data), false);
    }

    #[test]
    fn parse_updated_from_header() {
        let data = "\
# Organizationally Unique Identifiers (OUIs)
# Updated: 2026-10-17T03:14:54Z
# Count: oui-24 = 40333
000001 US Some Company
";
        let ts = parse_updated_reader(Cursor::new(data), "test");
        assert_eq!(ts, "2026-10-17T03:14:54Z");
    }

    #[test]
    #[should_panic(expected = "Missing '# Updated:' comment")]
    fn missing_updated_panics() {
        let data = "# other comment\n000001 US X\n";
        parse_updated_reader(Cursor::new(data), "test");
    }

    #[test]
    fn prefix_hex_parsing() {
        assert_eq!(parse_prefix_hex("000001"), vec![0x00, 0x00, 0x01]);
        assert_eq!(parse_prefix_hex("0001A2B"), vec![0x00, 0x01, 0xA2, 0xB0]);
        assert_eq!(parse_prefix_hex("AABBCCDD"), vec![0xAA, 0xBB, 0xCC, 0xDD]);
    }
}
