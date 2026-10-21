//! OUI 查询表的列式内存布局与编码（`no_std`）
//!
//! 本模块定义 bundled / owned 两种数据库共用的布局类型与 `OuiLoc` 编码，
//! 使二者的查表 / 迭代路径完全一致 (isomorphic layout)。

/// `loc` 列的编码类型 (32 位)。
///
/// ## Type 0 - 指向 `names` 字符串 (bit 31 = 0)
/// ```text
/// bit:  31 | 30 29 ... 11 10 09 08 | 07 06 ... 01 00
///       ──   ─────────────────────   ───────────────
///       0      offset (23 bits)        length (9 bits)
/// ```
///
/// ## Type 1 - 指向子表 (bit 31 = 1)
/// ```text
/// bit:  31 | 30 | 29 28 ... 01 00
///       ──   ──   ───────────────
///       1  type    subtable index
/// ```
///
/// * bit 31: 0 = names, 1 = subtable
///   - bit 31 = 0: `names` 内的 offset / length
///       - bit 30-09: offset (22 bits)
///       - bit 08-00: length (9 bits)
///   - bit 31 = 1: 指向子表
///       - bit 30: 子表类型 (0 = oui_28, 1 = oui_36)
///       - bit 29-00: 子表索引 (30 bits)
pub type OuiLoc = u32;

/// `loc` 指向 `names` 时的类型标志位 (bit 31 = 0)。
pub const LOC_NAMES_FLAG: OuiLoc = 0;
/// `loc` 指向子表时的类型标志位 (bit 31)。
pub const LOC_SUBTABLE_FLAG: OuiLoc = 1 << 31;

/// names offset 字段的掩码 (bit 30-09, 22 bits)。
pub const LOC_OFFSET_MASK: OuiLoc = 0x3F_FFFF << 9;
/// names length 字段的掩码 (bit 08-00, 9 bits)。
pub const LOC_LENGTH_MASK: OuiLoc = 0x1FF;

/// 子表索引字段的掩码 (bit 29-00, 30 bits)。
pub const SUBTABLE_INDEX_MASK: OuiLoc = 0x3FFF_FFFF;

/// 子表类型标志位：0 = oui_28, 1 = oui_36。
///
/// 1 表示 oui_36 (bit 30)。
pub const SUBTABLE_TYPE_36: OuiLoc = 1 << 30;

/// 编码一个指向 `names` 的 `loc`。
///
/// `offset` 最大 22 位 (0x3F_FFFF)，`length` 最大 9 位 (0x1FF)。
#[inline]
pub const fn encode_loc_names(offset: usize, length: usize) -> OuiLoc {
    ((offset as OuiLoc) << 9) | (length as OuiLoc)
}

/// 从指向 `names` 的 `loc` 解码出 `(offset, length)`。
#[inline]
pub const fn decode_loc_names(loc: OuiLoc) -> (usize, usize) {
    let offset = ((loc >> 9) & 0x3F_FFFF) as usize;
    let length = (loc & 0x1FF) as usize;
    (offset, length)
}

/// 编码一个指向子表的 `loc`。`subtable_type` 为 0 (oui_28) 或 1 (oui_36)。
#[inline]
pub const fn encode_loc_subtable(subtable_type: u32, subtable_idx: usize) -> OuiLoc {
    LOC_SUBTABLE_FLAG | (subtable_type << 30) | (subtable_idx as OuiLoc)
}

/// 子表描述符：指向前缀 / loc 内存池中的连续区间
/// (等价于 `&[T]` 加一个长度)。
///
/// bundled 与 owned 数据库共用。`u16` 长度足以覆盖协议上限，且由构造保证
/// `prefix_len == loc_len`：
/// - MA-M (28-bit)：表内键是第 4 字节的高 4 位 → 每表至多 16 项
/// - MA-S (36-bit)：表内键是第 4 字节加第 5 字节的高 4 位 (12 位) → 每表至多 4096 项
///
/// 字段重排后整个结构为 12 字节 (align 4)，而不是 4 × `u32` 的 16 字节。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtableDesc {
    /// 该子表前缀在 `prefix_pool_*` 中的起始下标
    pub prefix_start: u32,
    /// 该子表 loc 在 `loc_pool_*` 中的起始下标
    pub loc_start: u32,
    /// 前缀元素个数
    pub prefix_len: u16,
    /// loc 元素个数 (由构造保证等于 `prefix_len`)
    pub loc_len: u16,
}

impl SubtableDesc {
    /// 从内存池中取出该子表的前缀切片。
    #[inline]
    pub fn prefix_slice<'a, T>(&self, pool: &'a [T]) -> &'a [T] {
        &pool[self.prefix_start as usize..][..self.prefix_len as usize]
    }

    /// 从内存池中取出该子表的 loc 切片。
    #[inline]
    pub fn loc_slice<'a>(&self, pool: &'a [OuiLoc]) -> &'a [OuiLoc] {
        &pool[self.loc_start as usize..][..self.loc_len as usize]
    }
}
