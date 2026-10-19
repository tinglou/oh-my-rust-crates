//! # Organizationally Unique Identifiers (OUI) database
//!
//! ## 查询
//! 1. 取 mac 前 3 个字节，二分查找 `OuiDb::oui_24` `prefix`, 找不到返回 None，获取 `loc` 对应值
//! 2. 如 loc bit-31 = 0，则计算 offset/length，返回 `OuiDb::names[offset..offset+length]`
//! 2. 如 loc bit-31 = 1 且 bit-30 = 0，取 mac [第 4 个字节 & 0xF0], 查找 `OuiDb::oui_28`
//! 2. 如 loc bit-31 = 1 且 bit-30 = 1，取 mac [第 4 个字节，第 5 个字节 & 0xF0], 查找 `OuiDb::oui_36`
//!
use alloc::string::String;
use core::fmt::Write;

use crate::MacAddress;

const OUI_SUBTABLE: &str = "Ieee Registration Authority";

const OUI_VIRTUAL: [&str; 7] = [
    "VMware",
    "QEMU virtual NIC",
    "Bochs virtual NIC",
    "PearPC virtual NIC",
    "Cooperative Linux virtual NIC",
    "Oracle VirtualBox virtual NIC",
    "OpenStack",
];

/// # Organizationally Unique Identifiers (OUI) database
pub struct OuiDb {
    /// 数据源更新时间（来自数据文件头注释 `# Updated:`）
    pub updated: &'static str,

    pub(crate) names: &'static str,

    /// MA-L(24 bit prefix) 24 bit ≈1677 万 传统大厂/运营商
    pub(crate) oui_24: &'static OuiTable<3>,

    /// MA-M(28 bit prefix) 20 bit ≈104 万 中型设备商/模组厂
    pub(crate) oui_28: &'static [OuiSubtable<u8>],

    /// MA-S(36 bit prefix) 12 bit =4096 IoT/小微/专用硬件
    pub(crate) oui_36: &'static [OuiSubtable<u16>],
}

/// # Locate org name or subtable offset
///
/// ## Type 0 - 指向 names 字符串（bit 31 = 0）
/// ```text
/// bit:  31 | 30 29 ... 11 10 09 08 | 07 06 ... 01 00
///       ──   ─────────────────────   ───────────────
///       0      offset (23 bits)        length
/// ```
///
/// ## Type 1 - 指向子表偏移量（bit 31 = 1）
/// ```text
/// bit:  31 | 30 | 29 28 ... 01 00
///       ──   ──   ───────────────
///       1  type    subtable index
/// ```
///
/// * bit 31: 0=names, 1=子表
///   - bit 31 = 0 指向 OuiDb::names 中的偏移量和长度
///       - bit-30-08: offset (23 bits)
///       - bit-07-00: length (8 bits)
///   - bit 31 = 1 指向子表
///       - bit-30: subtable type
///         - 0 指向 OuiDb::oui_28
///         - 1 指向 OuiDb::oui_36
///       - bit-29-00: subtable index (30 bits)
///
type OuiLoc = u32;

/// `OuiEntry::extra_prefix` 编码：高 2 位为类型标志（bit-15-14），低 12 位为附加值
///
/// 附加值 = 子表 key >> 4（去掉 nibble 对齐用的低 4 个 0 位），因此：
/// - `0b00`：24 位前缀，无附加值
/// - `0b01`：28 位前缀，低 4 位为附加 nibble
/// - `0b10`：36 位前缀，低 12 位为附加值
const EXTRA_NONE: u16 = 0b00 << 14;
const EXTRA_OUI28: u16 = 0b01 << 14;
const EXTRA_OUI36: u16 = 0b10 << 14;

/// 列存储（节约内存，提升缓存利用率）
pub(crate) struct OuiTable<const N: usize> {
    pub(crate) prefix: &'static [[u8; N]],
    pub(crate) loc: &'static [OuiLoc],
}

/// 列存储（节约内存，提升缓存利用率）
pub(crate) struct OuiSubtable<T: 'static> {
    pub(crate) prefix: &'static [T],
    pub(crate) loc: &'static [OuiLoc],
}

pub struct OuiEntry<'a> {
    /// 24bit prefix
    pub(crate) prefix: &'a [u8],
    /// extra prefix for oui-28 or oui-36（高 2 位类型标志，低 12 位附加值 key>>4）
    pub(crate) extra_prefix: u16,

    pub(crate) name: &'a str,
    pub(crate) country_or_region_code: &'a str,
}

impl<'a> OuiEntry<'a> {
    /// 返回完整前缀的十六进制字符串（大写）：
    /// - 24 位：6 字符，如 `"000001"`
    /// - 28 位：7 字符，如 `"000001A"`
    /// - 36 位：9 字符，如 `"000001A2B"`
    ///
    /// 前缀位数由 [`OuiEntry::extra_prefix`] 的高 2 位标志决定。
    pub fn prefix_hex(&self) -> String {
        let mut s = String::with_capacity(9);
        for b in self.prefix {
            let _ = write!(s, "{b:02X}");
        }
        match self.extra_prefix & (0b11 << 14) {
            EXTRA_OUI28 => {
                let _ = write!(s, "{:X}", self.extra_prefix & 0x0F);
            }
            EXTRA_OUI36 => {
                let _ = write!(s, "{:03X}", self.extra_prefix & 0x0FFF);
            }
            _ => {}
        }
        s
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn country_or_region_code(&self) -> &str {
        self.country_or_region_code
    }
}

impl core::fmt::Debug for OuiEntry<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OuiEntry")
            .field("prefix", &self.prefix_hex())
            .field("name", &self.name)
            .field("country_or_region_code", &self.country_or_region_code)
            .finish()
    }
}

impl OuiDb {
    /// Checks if the given OUI name corresponds to a virtual NIC.
    ///
    /// # Arguments
    ///
    /// * `name` - The OUI name to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the name is a virtual NIC (e.g., QEMU, VirtualBox), otherwise `false`.
    pub fn is_virtual_nic(name: &str) -> bool {
        OUI_VIRTUAL.contains(&name)
    }

    /// Returns the name identifier of the subtable (`oui_28`, `oui_32`, `oui_36`) in the database.
    ///
    /// # Returns
    ///
    /// Returns the subtable name string `"Ieee Registration Authority"`.
    pub fn oui_subtable_name() -> &'static str {
        OUI_SUBTABLE
    }

    /// Returns the total size in bytes of the embedded static data referenced by this database.
    ///
    /// This sums the organization-name string, every lookup-table entry (the
    /// `prefix` and `loc` arrays), and both the subtable descriptors and their
    /// contents. It is useful for reporting the memory footprint of the OUI
    /// database (e.g. in documentation or diagnostics).
    ///
    /// # Example
    ///
    /// ```
    /// use macaddr_ouidb::OUI_DB;
    /// let bytes = OUI_DB.data_size_bytes();
    /// assert!(bytes > 0);
    /// ```
    pub fn data_size_bytes(&self) -> usize {
        let mut total = self.names.len();
        total += core::mem::size_of_val(self.oui_24.prefix);
        total += core::mem::size_of_val(self.oui_24.loc);
        total += core::mem::size_of_val(self.oui_28);
        for st in self.oui_28 {
            total += core::mem::size_of_val(st.prefix);
            total += core::mem::size_of_val(st.loc);
        }
        total += core::mem::size_of_val(self.oui_36);
        for st in self.oui_36 {
            total += core::mem::size_of_val(st.prefix);
            total += core::mem::size_of_val(st.loc);
        }
        total
    }

    /// Looks up the organization name (OUI vendor information) for a given MAC address.
    ///
    /// # Arguments
    ///
    /// * `mac` - The MAC address as a [`MacAddress`] type.
    ///
    /// # Returns
    ///
    /// Returns `Some(OuiEntry)`, otherwise `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use macaddr_ouidb::*;
    /// let mac = MacAddress::new([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
    /// let org = OUI_DB.lookup_mac(mac);
    /// ```
    pub fn lookup_mac(&self, mac: MacAddress) -> Option<OuiEntry<'static>> {
        self.lookup(mac.octets())
    }

    /// Looks up the organization entry for a given MAC address byte array.
    ///
    /// Supports 24-bit, 28-bit, and 36-bit OUI queries.
    ///
    /// # Arguments
    ///
    /// * `mac` - A 6-byte MAC address byte array.
    ///
    /// # Returns
    ///
    /// Returns `Some(OuiEntry)` if found, otherwise `None`.
    ///
    // # Query Logic
    //
    // 1. Extract the first 3 bytes and perform a binary search in `oui_24.prefix`
    // 2. Based on `loc` bit 31, determine the type:
    //    - `0`: Retrieve string from `names` using offset and length
    //    - `1` (bit 30 = 0): Look up in `oui_28` subtable using `mac[3] & 0xF0`
    //    - `1` (bit 30 = 1): Look up in `oui_36` subtable using `mac[3]` and `mac[4] & 0xF0` combined
    pub fn lookup(&self, mac: [u8; 6]) -> Option<OuiEntry<'static>> {
        let key3 = [mac[0], mac[1], mac[2]];
        let idx = self.oui_24.prefix.binary_search(&key3).ok()?;
        let loc = self.oui_24.loc[idx];
        let base: &'static [u8] = &self.oui_24.prefix[idx];

        if loc & (1 << 31) == 0 {
            // 24 位：loc 直接指向 names
            return Self::entry_from_loc(base, EXTRA_NONE, self.names, loc);
        }

        let sub_idx = (loc & 0x3FFF_FFFF) as usize;
        if loc & (1 << 30) == 0 {
            // 28 位：额外前缀直接取自 OuiSubtable<u8>::prefix
            let key4 = mac[3] & 0xF0;
            let sub = &self.oui_28[sub_idx];
            let i = sub.prefix.binary_search(&key4).ok()?;
            let extra = EXTRA_OUI28 | (sub.prefix[i] as u16 >> 4);
            Self::entry_from_loc(base, extra, self.names, sub.loc[i])
        } else {
            // 36 位：额外前缀直接取自 OuiSubtable<u16>::prefix
            let key5 = ((mac[3] as u16) << 8) | ((mac[4] & 0xF0) as u16);
            let sub = &self.oui_36[sub_idx];
            let i = sub.prefix.binary_search(&key5).ok()?;
            let extra = EXTRA_OUI36 | (sub.prefix[i] >> 4);
            Self::entry_from_loc(base, extra, self.names, sub.loc[i])
        }
    }

    /// 由 loc 解码 name/region，并构造 [`OuiEntry`]
    fn entry_from_loc(
        base: &'static [u8],
        extra_prefix: u16,
        names: &'static str,
        loc: OuiLoc,
    ) -> Option<OuiEntry<'static>> {
        let offset = ((loc >> 8) & 0x7F_FFFF) as usize;
        let length = (loc & 0xFF) as usize;
        let slice = names.get(offset..offset + length)?;
        if slice.len() < 2 {
            return None;
        }
        Some(OuiEntry {
            prefix: base,
            extra_prefix,
            name: &slice[2..],
            country_or_region_code: &slice[0..2],
        })
    }

    /// Returns an iterator over every [`OuiEntry`] in the database.
    ///
    /// The iterator walks the 24-bit table in order. Entries whose `loc`
    /// points directly at a name yield a 24-bit entry; entries that point at a
    /// subtable are expanded into their 28-bit or 36-bit entries.
    ///
    /// # Example
    ///
    /// ```
    /// use macaddr_ouidb::OUI_DB;
    /// let count = OUI_DB.iter().count();
    /// assert!(count > 40_000);
    /// ```
    pub fn iter(&self) -> OuiIter<'_> {
        OuiIter {
            db: self,
            table_i: 0,
            sub: None,
        }
    }

    /// 对数据库中的每一条 [`OuiEntry`] 调用处理函数 `f`（副作用式，等价于
    /// `for e in db.iter() { f(e) }`）。
    ///
    /// 与 [`OuiDb::iter`] 覆盖范围一致：24 位名称条目、以及各 24 位前缀下
    /// 展开的 28/36 位子表条目。
    ///
    /// # Example
    ///
    /// ```
    /// use macaddr_ouidb::OUI_DB;
    /// let mut count = 0usize;
    /// OUI_DB.for_each(|_e| count += 1);
    /// assert_eq!(count, OUI_DB.iter().count());
    /// ```
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(OuiEntry<'static>),
    {
        for entry in self.iter() {
            f(entry);
        }
    }
}

/// 当前正在遍历的 28/36 位子表状态
struct SubState {
    is36: bool,
    base: &'static [u8],
    sub_idx: usize,
    entry_idx: usize,
}

/// Iterator over all [`OuiEntry`] values in an [`OuiDb`].
///
/// Created by [`OuiDb::iter`].
pub struct OuiIter<'a> {
    db: &'a OuiDb,
    table_i: usize,
    sub: Option<SubState>,
}

impl Iterator for OuiIter<'_> {
    type Item = OuiEntry<'static>;

    fn next(&mut self) -> Option<OuiEntry<'static>> {
        loop {
            if let Some(sub) = self.sub.as_mut() {
                if sub.is36 {
                    let st = &self.db.oui_36[sub.sub_idx];
                    if sub.entry_idx < st.loc.len() {
                        let i = sub.entry_idx;
                        sub.entry_idx += 1;
                        let extra_prefix = EXTRA_OUI36 | (st.prefix[i] >> 4);
                        if let Some(e) =
                            OuiDb::entry_from_loc(sub.base, extra_prefix, self.db.names, st.loc[i])
                        {
                            return Some(e);
                        }
                    } else {
                        self.sub = None;
                    }
                } else {
                    let st = &self.db.oui_28[sub.sub_idx];
                    if sub.entry_idx < st.loc.len() {
                        let i = sub.entry_idx;
                        sub.entry_idx += 1;
                        let extra_prefix = EXTRA_OUI28 | (st.prefix[i] as u16 >> 4);
                        if let Some(e) =
                            OuiDb::entry_from_loc(sub.base, extra_prefix, self.db.names, st.loc[i])
                        {
                            return Some(e);
                        }
                    } else {
                        self.sub = None;
                    }
                }
                continue;
            }

            if self.table_i >= self.db.oui_24.loc.len() {
                return None;
            }

            let idx = self.table_i;
            self.table_i += 1;
            let loc = self.db.oui_24.loc[idx];
            let base: &'static [u8] = &self.db.oui_24.prefix[idx];

            if loc & (1 << 31) == 0 {
                if let Some(e) = OuiDb::entry_from_loc(base, EXTRA_NONE, self.db.names, loc) {
                    return Some(e);
                }
            } else {
                self.sub = Some(SubState {
                    is36: loc & (1 << 30) != 0,
                    base,
                    sub_idx: (loc & 0x3FFF_FFFF) as usize,
                    entry_idx: 0,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_hex_all_widths() {
        let base: &[u8] = &[0x00, 0x1A, 0x2B];

        let e24 = OuiEntry {
            prefix: base,
            extra_prefix: EXTRA_NONE,
            name: "x",
            country_or_region_code: "US",
        };
        assert_eq!(e24.prefix_hex(), "001A2B");

        let e28 = OuiEntry {
            prefix: base,
            extra_prefix: EXTRA_OUI28 | 0x0A,
            name: "x",
            country_or_region_code: "US",
        };
        assert_eq!(e28.prefix_hex(), "001A2BA");

        let e36 = OuiEntry {
            prefix: base,
            extra_prefix: EXTRA_OUI36 | 0x0AB,
            name: "x",
            country_or_region_code: "US",
        };
        assert_eq!(e36.prefix_hex(), "001A2B0AB");

        // 附加值最高位为 1 时也不能与类型标志冲突
        let e36_hi = OuiEntry {
            prefix: base,
            extra_prefix: EXTRA_OUI36 | 0xC0D,
            name: "x",
            country_or_region_code: "US",
        };
        assert_eq!(e36_hi.prefix_hex(), "001A2BC0D");
    }

    #[test]
    fn lookup_returns_oui_entry() {
        let mac = crate::MacAddress::new([0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
        let entry = crate::OUI_DB.lookup_mac(mac).expect("XEROX entry");
        assert_eq!(entry.prefix_hex(), "000001");
        assert_eq!(entry.country_or_region_code(), "US");
        assert!(entry.name().starts_with("XEROX"));
    }

    #[test]
    fn iter_yields_all_entry_widths() {
        let db = &crate::OUI_DB;
        let mut count = 0usize;
        let (mut c24, mut c28, mut c36) = (0usize, 0usize, 0usize);
        for e in db.iter() {
            count += 1;
            assert_eq!(e.country_or_region_code().len(), 2);
            match e.prefix_hex().len() {
                6 => c24 += 1,
                7 => c28 += 1,
                9 => c36 += 1,
                other => panic!("unexpected prefix length {other}"),
            }
        }

        let expect_24 = db
            .oui_24
            .loc
            .iter()
            .filter(|l| **l & (1 << 31) == 0)
            .count();
        let expect_28: usize = db.oui_28.iter().map(|s| s.loc.len()).sum();
        let expect_36: usize = db.oui_36.iter().map(|s| s.loc.len()).sum();

        assert_eq!(c24, expect_24);
        assert_eq!(c28, expect_28);
        assert_eq!(c36, expect_36);
        assert_eq!(count, expect_24 + expect_28 + expect_36);
    }

    #[test]
    fn iter_and_lookup_roundtrip() {
        let db = &crate::OUI_DB;
        for e in db.iter() {
            let mut mac = [0u8; 6];
            mac[..3].copy_from_slice(e.prefix);
            match e.extra_prefix & (0b11 << 14) {
                EXTRA_OUI28 => mac[3] = ((e.extra_prefix & 0x0F) as u8) << 4,
                EXTRA_OUI36 => {
                    let v = e.extra_prefix & 0x0FFF;
                    mac[3] = (v >> 4) as u8;
                    mac[4] = ((v & 0x0F) as u8) << 4;
                }
                _ => {}
            }
            let looked = crate::OUI_DB
                .lookup(mac)
                .unwrap_or_else(|| panic!("lookup failed for {}", e.prefix_hex()));
            assert_eq!(looked.prefix_hex(), e.prefix_hex());
            assert_eq!(looked.name(), e.name());
        }
    }

    #[test]
    fn for_each_visits_every_entry() {
        let db = &crate::OUI_DB;
        let mut names: Vec<String> = Vec::new();
        db.for_each(|e| names.push(e.name().to_string()));
        let expect: Vec<String> = db.iter().map(|e| e.name().to_string()).collect();
        assert_eq!(names.len(), db.iter().count());
        assert_eq!(names, expect);
    }

    /// The embedded database (names + lookup tables) currently occupies roughly
    /// 1.0 MB with trimmed names and ~1.2 MB with the `fullname` dataset. This
    /// test guards against accidental runaway growth while tolerating normal
    /// data refreshes.
    #[test]
    fn data_size_is_within_expected_range() {
        let db = &crate::OUI_DB;
        let size = db.data_size_bytes();

        // Always covers at least the name string itself.
        assert!(size >= db.names.len());
        assert!(
            size > 512 * 1024,
            "database unexpectedly small: {size} bytes"
        );
        assert!(
            size < 2 * 1024 * 1024,
            "database unexpectedly large: {size} bytes"
        );
    }

    #[test]
    fn data_size_is_deterministic() {
        let db = &crate::OUI_DB;
        assert_eq!(db.data_size_bytes(), db.data_size_bytes());
    }

    #[test]
    fn updated_timestamp_is_present() {
        let db = &crate::OUI_DB;
        // RFC 3339: 2026-10-17T03:14:54Z
        assert!(!db.updated.is_empty());
        assert!(db.updated.ends_with('Z'));
        assert_eq!(db.updated.len(), 20);
    }
}
