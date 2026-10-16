//! # Organizationally Unique Identifiers (OUI) database
//!
//! ## 查询
//! 1. 取 mac 前 3 个字节，二分查找 `OuiDb::oui_24` `prefix`, 找不到返回 None，获取 `loc` 对应值
//! 2. 如 loc bit-31-30 = 0，则计算 offset/length，返回 `OuiDb::names[offset..offset+length]`
//! 2. 如 loc bit-31-30 = 1，取 mac [第 4 个字节 & 0xF0], 查找 `OuiDb::oui_28`
//! 2. 如 loc bit-31-30 = 2，取 mac [第 4 个字节，第 5 个字节 & 0xF0], 查找 `OuiDb::oui_36`
//!
use crate::MacAddress;

const OUI_SUBTABLE: &str = "Ieee Registration Authority";

const OUI_VIRTUAL: [&'static str; 7] = [
    "VMware",
    "QEMU virtual NIC",
    "Bochs virtual NIC",
    "PearPC virtual NIC",
    "Cooperative Linux virtual NIC",
    "Oracle VirtualBox virtual NIC",
    "OpenStack",
];

/// # Organizationally Unique Identifiers (OUI) database
///

pub struct OuiDb {
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
/// ## Type 00 - 指向 names 字符串
/// ```text
/// bit:  31 30 | 29 28 ... 11 10 09 08 | 07 06 ... 01 00
///       ─────   ─────────────────────   ───────────────
///       type       offset (22 bits)        length
/// ```
///
/// ## Type 01/02/03 - 指向子表偏移量
/// ```text
/// bit:  31 30 | 29 28 ... 17 16 | 15 14 ... 01 00
///       ─────   ───────────────   ───────────────
///       type       reserved        subtable index
/// ```
///
/// * bit 31-30: type
///   - 00 指向 OuiDb::names 中的偏移量和长度
///       - bit-29-08: offset (22 bits)
///       - bit-07-00: length (8 bits)
///   - 01 指向 OuiDb::oui_28 偏移量
///       - bit-15-00: subtable index (16 bits)
///   - 02 指向 OuiDb::oui_36 偏移量
///       - bit-15-00: subtable index (16 bits)
///
type OuiLoc = u32;

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
    /// Returns `Some((&str, &str))` with (name, region), otherwise `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use macaddr_ouidb::*;
    /// let mac = MacAddress::new([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
    /// let org = OUI_DB.lookup_mac(mac);
    /// ```
    pub fn lookup_mac(&self, mac: MacAddress) -> Option<(&'static str, &'static str)> {
        self.lookup(mac.octets())
    }

    /// Looks up the organization name and region for a given MAC address byte array.
    ///
    /// Supports 24-bit, 28-bit, and 36-bit OUI queries.
    ///
    /// # Arguments
    ///
    /// * `mac` - A 6-byte MAC address byte array.
    ///
    /// # Returns
    ///
    /// Returns `Some((&str, &str))` if found, otherwise `None`.
    ///
    // # Query Logic
    //
    // 1. Extract the first 3 bytes and perform a binary search in `oui_24.prefix`
    // 2. Based on `loc` bits 31-30, determine the type:
    //    - `00`: Retrieve string from `names` using offset and length
    //    - `01`: Look up in `oui_28` subtable using `mac[3] & 0xF0`
    //    - `02`: Look up in `oui_36` subtable using `mac[3]` and `mac[4] & 0xF0` combined
    pub fn lookup(&self, mac: [u8; 6]) -> Option<(&'static str, &'static str)> {
        let key3 = [mac[0], mac[1], mac[2]];
        let idx = match self.oui_24.prefix.binary_search(&key3) {
            Ok(idx) => idx,
            Err(_) => return None,
        };
        let loc = self.oui_24.loc[idx];
        match (loc >> 30) & 0x03 {
            0 => {
                let offset = ((loc >> 8) & 0x3FFFFF) as usize;
                let length = (loc & 0xFF) as usize;
                let slice = self.names.get(offset..offset + length)?;
                if slice.len() < 2 {
                    return None;
                }
                let region = &slice[0..2];
                let name = &slice[2..];
                Some((name, region))
            }
            1 => {
                let sub_idx = (loc & 0xFFFF) as usize;
                let key4 = mac[3] & 0xF0;
                self.lookup_subtable(&self.oui_28[sub_idx], key4)
            }
            2 => {
                let sub_idx = (loc & 0xFFFF) as usize;
                let key5 = ((mac[3] as u16) << 8) | ((mac[4] & 0xF0) as u16);
                self.lookup_subtable(&self.oui_36[sub_idx], key5)
            }
            _ => None,
        }
    }

    fn lookup_subtable<T: Ord + Copy>(
        &self,
        subtable: &OuiSubtable<T>,
        key: T,
    ) -> Option<(&'static str, &'static str)> {
        let idx = match subtable.prefix.binary_search(&key) {
            Ok(idx) => idx,
            Err(_) => return None,
        };
        let loc = subtable.loc[idx];
        let offset = ((loc >> 8) & 0x3FFFFF) as usize;
        let length = (loc & 0xFF) as usize;
        let slice = self.names.get(offset..offset + length)?;
        if slice.len() < 2 {
            return None;
        }
        let region = &slice[0..2];
        let name = &slice[2..];
        Some((name, region))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(size > 512 * 1024, "database unexpectedly small: {size} bytes");
        assert!(size < 2 * 1024 * 1024, "database unexpectedly large: {size} bytes");
    }

    #[test]
    fn data_size_is_deterministic() {
        let db = &crate::OUI_DB;
        assert_eq!(db.data_size_bytes(), db.data_size_bytes());
    }
}

