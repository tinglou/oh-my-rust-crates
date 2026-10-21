//! OUI 查询表的列式内存布局与编码（`no_std`）
//!
//! 本模块定义 bundled / owned 两种数据库共用的布局类型与 `OuiLoc` 编码，
//! 使二者的查表 / 迭代路径完全一致 (isomorphic layout)。

/// `loc` 列的编码类型 (32 位)。
///
/// 使用“高位字节判别”方案：`bit 31-24` 全为 `1` (`0xFF`) 表示指向子表，
/// 否则表示指向 `names` 字符串。
///
/// ## Type 0 - 指向 `names` 字符串 (高位字节 != 0xFF)
/// ```text
/// bit:  31 30 ... 09 08 | 07 06 ... 01 00
///       ───────────────   ───────────────
///        offset (23 bits)   length (9 bits)
/// ```
///
/// ## Type 1 - 指向子表 (bit 31-24 = 0xFF)
/// ```text
/// bit:  31 ... 24 | 23 ... 13 | 12 | 11 ... 00
///       ─────────   ─────────   ──   ─────────
///         0xFF       保留         type  subtable index
/// ```
///
/// * 高位字节 `== 0xFF`: 指向子表
///   - bit 12: 子表类型 (0 = oui_28, 1 = oui_36)
///   - bit 11-00: 子表索引 (12 bits, 上限 4095)
///   - bit 23-13: 保留 (置 0)
/// * 高位字节 `!= 0xFF`: `names` 内的 offset / length
///   - bit 31-09: offset (23 bits, 上限 0x7F_FFFF)
///   - bit 08-00: length (9 bits, 上限 0x1FF)
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OuiLoc(u32);

/// 子表分支的高位字节标记 (bit 31-24 全 1)。
const LOC_SUBTABLE_TAG: u32 = 0xFF00_0000;

/// names offset 字段的移位 (offset 位于 bit 31-09)。
const LOC_OFFSET_SHIFT: u32 = 9;
/// names offset 字段的掩码 (bit 31-09)。
const LOC_OFFSET_MASK: u32 = 0x7F_FF_FF << LOC_OFFSET_SHIFT;
/// names length 字段的掩码 (bit 08-00)。
const LOC_LENGTH_MASK: u32 = 0x1FF;

/// 子表索引字段的掩码 (bit 11-00)。
const SUBTABLE_INDEX_MASK: u32 = 0x0FFF;
/// 子表类型标志位：1 表示 oui_36 (bit 12)。
const SUBTABLE_TYPE_36: u32 = 1 << 12;

impl OuiLoc {
    /// names offset 上限 (23 bits)。
    pub const MAX_OFFSET: usize = 0x7F_FFFF;
    /// names length 上限 (9 bits)。
    pub const MAX_LENGTH: usize = 0x1FF;
    /// 子表索引上限 (12 bits)。
    pub const MAX_SUBTABLE_INDEX: usize = 0x0FFF;

    /// 从原始位模式构造 (供 codegen 生成字面量使用)。
    ///
    /// `const fn`：可在 const 上下文（如生成的 `const &[OuiLoc]` 数组）中调用。
    /// 非 const 上下文亦可使用 `OuiLoc::from(bits)` / `bits.into()`。
    #[inline]
    pub const fn new(bits: u32) -> OuiLoc {
        OuiLoc(bits)
    }

    /// 取出原始位模式。
    #[inline]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// 编码一个指向 `names` 的 `loc`。
    ///
    /// `offset` 最大 23 位 ([`OuiLoc::MAX_OFFSET`])，`length` 最大 9 位 ([`OuiLoc::MAX_LENGTH`])。
    #[inline]
    pub fn names(offset: usize, length: usize) -> OuiLoc {
        debug_assert!(offset <= Self::MAX_OFFSET, "Offset too large: {offset}");
        debug_assert!(length <= Self::MAX_LENGTH, "Length too large: {length}");
        Self::new(((offset as u32) << LOC_OFFSET_SHIFT) | (length as u32))
    }

    /// 编码一个指向子表的 `loc`。
    ///
    /// `is_36` 为 `false` 表示 oui_28、`true` 表示 oui_36；`subtable_idx` 最大 12 位
    /// ([`OuiLoc::MAX_SUBTABLE_INDEX`])。
    #[inline]
    pub fn subtable(is_36: bool, subtable_idx: usize) -> OuiLoc {
        debug_assert!(
            subtable_idx <= Self::MAX_SUBTABLE_INDEX,
            "Subtable index too large: {subtable_idx}"
        );
        let type_bit = if is_36 { SUBTABLE_TYPE_36 } else { 0 };
        Self::new(LOC_SUBTABLE_TAG | type_bit | (subtable_idx as u32))
    }

    /// 是否指向子表 (高位字节 == 0xFF)。
    #[inline]
    pub const fn is_subtable(self) -> bool {
        (self.0 >> 24) == 0xFF
    }

    /// 解码指向 `names` 的 `(offset, length)`。
    #[inline]
    pub const fn decode_names(self) -> (usize, usize) {
        let offset = ((self.0 & LOC_OFFSET_MASK) >> LOC_OFFSET_SHIFT) as usize;
        let length = (self.0 & LOC_LENGTH_MASK) as usize;
        (offset, length)
    }

    /// 子表索引 (bit 11-00)。
    #[inline]
    pub const fn subtable_index(self) -> usize {
        (self.0 & SUBTABLE_INDEX_MASK) as usize
    }

    /// 子表是否为 oui_36。
    #[inline]
    pub const fn subtable_is_36(self) -> bool {
        (self.0 & SUBTABLE_TYPE_36) != 0
    }
}

impl From<u32> for OuiLoc {
    #[inline]
    fn from(bits: u32) -> OuiLoc {
        OuiLoc(bits)
    }
}

impl From<OuiLoc> for u32 {
    #[inline]
    fn from(loc: OuiLoc) -> u32 {
        loc.0
    }
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
    /// 构造一个子表描述符，`prefix_len == loc_len == len`。
    ///
    /// debug 构建下校验 `len` 不超过子表索引上限 ([`OuiLoc::MAX_SUBTABLE_INDEX`] + 1)。
    #[inline]
    pub fn new(prefix_start: u32, loc_start: u32, len: usize) -> SubtableDesc {
        debug_assert!(
            len <= OuiLoc::MAX_SUBTABLE_INDEX + 1,
            "subtable too large: {len}"
        );
        let len = len as u16;
        SubtableDesc {
            prefix_start,
            loc_start,
            prefix_len: len,
            loc_len: len,
        }
    }

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
