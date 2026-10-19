//! OUI 中间数据结构构建
//!
//! 负责读取 custom / ieee 两个数据文件、合并去重、解析 `# Updated:` 时间戳，
//! 并构建为查表用的中间数据结构（names 字典、24 位主表、28/36 位子表），
//! 供调用方（如 `build.rs`）生成目标代码。

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::path::Path;

use crate::{OuiEntry, parse_oui_file, parse_prefix_hex, parse_updated_reader};

/// 子表引用
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum SubtableRef {
    Oui28(usize),
    Oui36(usize),
}

/// 28 位子表条目
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Oui28Entry {
    /// 高 4 位有效
    pub prefix: u8,
    pub name_idx: usize,
}

/// 36 位子表条目
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Oui36Entry {
    /// 高 12 位有效：高 8 位是第 4 字节，低 4 位是第 5 字节的高 4 位
    pub prefix: u16,
    pub name_idx: usize,
}

/// 构建中间数据结构
#[derive(Debug, Default)]
pub struct OuiDataBuilder {
    /// 数据源更新时间（来自 ieee 文件头注释 `# Updated:`）
    pub updated: String,
    /// 所有唯一名称的列表（用于生成 names 字符串）
    pub names: Vec<String>,
    /// 名称到索引的映射
    pub name_to_idx: BTreeMap<String, usize>,
    /// 24 位表数据：prefix -> (name_idx, subtable)
    pub oui24_map: BTreeMap<[u8; 3], (usize, Option<SubtableRef>)>,
    /// 28 位子表列表
    pub oui28_tables: Vec<Vec<Oui28Entry>>,
    /// 28 位子表索引到 24 位前缀的映射
    pub oui28_idx_to_prefix: BTreeMap<usize, [u8; 3]>,
    /// 36 位子表列表
    pub oui36_tables: Vec<Vec<Oui36Entry>>,
    /// 36 位子表索引到 24 位前缀的映射
    pub oui36_idx_to_prefix: BTreeMap<usize, [u8; 3]>,
    /// 记录只有子表没有 24 位条目的前缀（用于日志）
    pub subtable_only_prefixes: Vec<([u8; 3], SubtableRef, usize)>,
}

impl OuiDataBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 custom / ieee 两个数据文件构建中间数据结构。
    ///
    /// - custom 优先；与 ieee 出现相同 `(bit_len, prefix)` 时返回错误（不允许静默覆盖）
    /// - 同时从 ieee 文件头注释解析 `# Updated:` 时间戳到 [`OuiDataBuilder::updated`]
    /// - ieee 文件内部的重复前缀已由 ouidb-builder 在生成时去重，此处不再处理
    pub fn build(&mut self, custom_path: &Path, ieee_path: &Path) -> Result<(), String> {
        let custom_name = file_name(custom_path);
        let ieee_name = file_name(ieee_path);

        let entries = Self::merge_entries(
            parse_oui_file(custom_path),
            parse_oui_file(ieee_path),
            &custom_name,
            &ieee_name,
        )?;

        self.updated = Self::parse_updated_timestamp(ieee_path);
        self.build_from_entries(entries);
        Ok(())
    }

    /// 获取或添加名称，返回索引
    fn get_or_add_name(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.name_to_idx.get(name) {
            idx
        } else {
            let idx = self.names.len();
            self.names.push(name.to_string());
            self.name_to_idx.insert(name.to_string(), idx);
            idx
        }
    }

    /// 合并 custom 与 ieee 条目，custom 优先。
    ///
    /// `custom_name` / `ieee_name` 仅用于错误信息显示。
    fn merge_entries(
        custom_entries: Vec<OuiEntry>,
        ieee_entries: Vec<OuiEntry>,
        custom_name: &str,
        ieee_name: &str,
    ) -> Result<Vec<OuiEntry>, String> {
        let mut entries = custom_entries;
        let custom_keys: HashSet<(usize, String)> = entries
            .iter()
            .map(|e| (e.bit_len, e.prefix_str.clone()))
            .collect();

        for e in ieee_entries {
            let key = (e.bit_len, e.prefix_str.clone());
            if custom_keys.contains(&key) {
                let custom_org = entries
                    .iter()
                    .find(|x| x.bit_len == e.bit_len && x.prefix_str == e.prefix_str)
                    .map(|x| x.org_name.clone())
                    .unwrap_or_default();
                return Err(format!(
                    "error: prefix {} ({} bits) appears in both {} and {}\n  custom entry org: {:?}\n  ieee entry org:   {:?}",
                    e.prefix_str, e.bit_len, custom_name, ieee_name, custom_org, e.org_name
                ));
            }
            entries.push(e);
        }
        Ok(entries)
    }

    /// 解析文件头注释中的 `# Updated: <timestamp>` 行，返回时间戳字符串
    ///
    /// 只扫描文件开头连续的 `#` 注释行，遇到非注释行即停止。
    fn parse_updated_timestamp(path: &Path) -> String {
        let file = fs::File::open(path)
            .unwrap_or_else(|e| panic!("Failed to open {}: {e}", path.display()));
        parse_updated_reader(BufReader::new(file), &path.display().to_string())
    }

    /// 从已解析条目构建所有数据结构（供 [`OuiDataBuilder::build`] 及测试使用）
    pub fn build_from_entries(&mut self, entries: Vec<OuiEntry>) {
        // 按前缀长度分组
        // entries_24: 3 字节前缀 -> 名称列表
        // entries_28: 3 字节前缀 -> (4 字节高 4 位，名称) 列表
        // entries_36: 3 字节前缀 -> (第 4 字节 + 第 5 字节高 4 位，名称) 列表
        let mut entries_24: BTreeMap<[u8; 3], Vec<(usize, String)>> = BTreeMap::new();
        let mut entries_28: BTreeMap<[u8; 3], Vec<(u8, usize)>> = BTreeMap::new();
        let mut entries_36: BTreeMap<[u8; 3], Vec<(u16, usize)>> = BTreeMap::new();

        for entry in entries {
            let name_idx = self.get_or_add_name(&entry.org_name);
            let bytes = parse_prefix_hex(&entry.prefix_str);

            match entry.bit_len {
                24 => {
                    let prefix: [u8; 3] = [bytes[0], bytes[1], bytes[2]];
                    entries_24
                        .entry(prefix)
                        .or_default()
                        .push((name_idx, entry.org_name));
                }
                28 => {
                    // 按 3 字节前缀分组，存储第 4 字节的高 4 位
                    let prefix3: [u8; 3] = [bytes[0], bytes[1], bytes[2]];
                    let key4 = bytes[3] & 0xF0;
                    entries_28
                        .entry(prefix3)
                        .or_default()
                        .push((key4, name_idx));
                }
                36 => {
                    // 按 3 字节前缀分组，存储第 4 字节 + 第 5 字节高 4 位
                    let prefix3: [u8; 3] = [bytes[0], bytes[1], bytes[2]];
                    let key5 = ((bytes[3] as u16) << 8) | ((bytes[4] & 0xF0) as u16);
                    entries_36
                        .entry(prefix3)
                        .or_default()
                        .push((key5, name_idx));
                }
                _ => {}
            }
        }

        // 构建 28 位子表：每个 3 字节前缀对应一个子表
        let mut oui24_to_subtable: BTreeMap<[u8; 3], SubtableRef> = BTreeMap::new();

        for (prefix3, entries_list) in entries_28 {
            let subtable_idx = self.oui28_tables.len();
            let entry_count = entries_list.len(); // 先保存长度
            let mut table: Vec<Oui28Entry> = entries_list
                .into_iter()
                .map(|(key4, name_idx)| Oui28Entry {
                    prefix: key4,
                    name_idx,
                })
                .collect();
            table.sort();
            table.dedup_by(|a, b| a.prefix == b.prefix);
            self.oui28_tables.push(table);
            self.oui28_idx_to_prefix.insert(subtable_idx, prefix3);

            oui24_to_subtable.insert(prefix3, SubtableRef::Oui28(subtable_idx));

            // 只有当 entries_24 中不存在该前缀时，才记录为 subtable_only
            if !entries_24.contains_key(&prefix3) {
                self.subtable_only_prefixes.push((
                    prefix3,
                    SubtableRef::Oui28(subtable_idx),
                    entry_count,
                ));
            }
        }

        // 构建 36 位子表：每个 3 字节前缀对应一个子表
        for (prefix3, entries_list) in entries_36 {
            let subtable_idx = self.oui36_tables.len();
            let entry_count = entries_list.len(); // 先保存长度
            let mut table: Vec<Oui36Entry> = entries_list
                .into_iter()
                .map(|(key5, name_idx)| Oui36Entry {
                    prefix: key5,
                    name_idx,
                })
                .collect();
            table.sort();
            table.dedup_by(|a, b| a.prefix == b.prefix);
            self.oui36_tables.push(table);
            self.oui36_idx_to_prefix.insert(subtable_idx, prefix3);

            oui24_to_subtable.insert(prefix3, SubtableRef::Oui36(subtable_idx));

            // 只有当 entries_24 中不存在该前缀时，才记录为 subtable_only
            if !entries_24.contains_key(&prefix3) {
                self.subtable_only_prefixes.push((
                    prefix3,
                    SubtableRef::Oui36(subtable_idx),
                    entry_count,
                ));
            }
        }

        // 构建 24 位表：合并所有 3 字节前缀（包括只有子表的前缀）
        let mut all_prefixes: BTreeMap<[u8; 3], Option<usize>> = BTreeMap::new();

        // 添加 24 位条目的前缀
        for (prefix, name_list) in &entries_24 {
            let name_idx = name_list[0].0;
            all_prefixes.insert(*prefix, Some(name_idx));
        }

        // 添加只有子表的前缀（使用子表中第一个条目的名称）
        for (prefix, subtable_ref) in &oui24_to_subtable {
            if !all_prefixes.contains_key(prefix) {
                // 从子表中获取第一个名称
                let name_idx = match subtable_ref {
                    SubtableRef::Oui28(idx) => self.oui28_tables[*idx].first().map(|e| e.name_idx),
                    SubtableRef::Oui36(idx) => self.oui36_tables[*idx].first().map(|e| e.name_idx),
                };
                all_prefixes.insert(*prefix, name_idx);
            }
        }

        // 构建 oui_24_map
        for (prefix, name_idx_opt) in all_prefixes {
            let name_idx = name_idx_opt.unwrap_or(0);
            let subtable = oui24_to_subtable.get(&prefix).copied();
            self.oui24_map.insert(prefix, (name_idx, subtable));
        }
    }
}

/// 提取路径文件名用于错误信息显示
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(prefix: &str, org: &str, bit_len: usize) -> OuiEntry {
        OuiEntry {
            prefix_str: prefix.into(),
            org_name: org.into(),
            bit_len,
        }
    }

    #[test]
    fn build_24bit_table() {
        let mut b = OuiDataBuilder::new();
        b.build_from_entries(vec![
            entry("000001", "USFoo", 24),
            entry("000001", "USBar", 24),
            entry("000002", "USFoo", 24),
        ]);
        assert_eq!(b.oui24_map.len(), 2);
        assert_eq!(b.names, vec!["USFoo", "USBar"]);
        let (name_idx, sub) = b.oui24_map[&[0x00, 0x00, 0x01]];
        assert_eq!(name_idx, 0); // 第一个名称
        assert!(sub.is_none());
    }

    #[test]
    fn build_subtables_and_subtable_only_prefix() {
        let mut b = OuiDataBuilder::new();
        b.build_from_entries(vec![
            entry("000001", "USFoo", 24),
            entry("0000010", "USBaz", 28),
            entry("000002123", "USQux", 36),
        ]);

        // 28 位子表
        assert_eq!(b.oui28_tables.len(), 1);
        assert_eq!(b.oui28_tables[0].len(), 1);
        assert_eq!(b.oui28_tables[0][0].prefix, 0x00);
        assert_eq!(b.oui28_idx_to_prefix[&0], [0x00, 0x00, 0x01]);

        // 36 位子表（该前缀只有子表）
        assert_eq!(b.oui36_tables.len(), 1);
        assert_eq!(b.oui36_idx_to_prefix[&0], [0x00, 0x00, 0x02]);

        // 24 位表包含纯子表前缀，name 取子表第一个条目
        assert_eq!(b.oui24_map.len(), 2);
        let (name_idx, sub) = b.oui24_map[&[0x00, 0x00, 0x02]];
        assert_eq!(b.names[name_idx], "USQux");
        assert_eq!(sub, Some(SubtableRef::Oui36(0)));

        // subtable-only 记录
        assert_eq!(b.subtable_only_prefixes.len(), 1);
        assert_eq!(b.subtable_only_prefixes[0].0, [0x00, 0x00, 0x02]);
    }

    #[test]
    fn merge_custom_wins_and_conflicts_error() {
        let custom = vec![entry("000001", "USCustom", 24)];
        let ieee = vec![
            entry("000002", "USIeee", 24),
            entry("000001", "USConflict", 24),
        ];

        // 无冲突时合并，custom 在前
        let ok = OuiDataBuilder::merge_entries(
            custom.clone(),
            ieee[..1].to_vec(),
            "custom-mac-oui.txt",
            "ieee-mac-oui.txt",
        )
        .unwrap();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[0].org_name, "USCustom");
        assert_eq!(ok[1].org_name, "USIeee");

        // 冲突时报错
        let err = OuiDataBuilder::merge_entries(
            custom,
            ieee.clone(),
            "custom-mac-oui.txt",
            "ieee-mac-oui.txt",
        )
        .unwrap_err();
        assert!(err.contains("000001"));
        assert!(err.contains("USConflict"));
        assert!(err.contains("custom-mac-oui.txt"));
        assert!(err.contains("ieee-mac-oui.txt"));

        // 不同 bit_len 的相同前缀不冲突
        let custom_28 = vec![entry("000001", "USCustom28", 28)];
        assert!(
            OuiDataBuilder::merge_entries(
                custom_28,
                ieee,
                "custom-mac-oui.txt",
                "ieee-mac-oui.txt"
            )
            .is_ok()
        );
    }
}
