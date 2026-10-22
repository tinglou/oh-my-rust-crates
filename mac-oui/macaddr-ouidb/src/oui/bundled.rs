//! Bundled (compile-time generated) OUI database

use macaddr_ouidb_support::layout::SubtableDesc;

use super::{
    EntrySource, OuiBackend, OuiEntry, OuiIter, OuiIterInner, OuiLoc, OuiTableIter, lookup_impl,
};

/// # Organizationally Unique Identifiers (OUI) database (bundled)
///
/// Data is generated at compile time by `build.rs`, with zero runtime
/// initialization cost.
///
/// The 28/36-bit subtables use the same memory layout as
/// [`OuiDbOwned`](super::OuiDbOwned): a table of [`SubtableDesc`] descriptors plus
/// contiguous `prefix` / `loc` memory pools. The only difference from the owned
/// database is the storage medium (`&'static` slices embedded in `.rodata`
/// versus `Vec`/`String` on the heap).
#[derive(Clone, Copy)]
pub struct OuiDbBundled {
    /// Data source update time (from the `# Updated:` header comment in the data file)
    pub updated: &'static str,

    pub(crate) names: &'static str,

    /// MA-L (24-bit prefix), 24 bits ≈ 16.77M, legacy large vendors / carriers
    pub(crate) oui_24_prefix: &'static [[u8; 3]],
    pub(crate) oui_24_loc: &'static [OuiLoc],

    /// MA-L registration dates (parallel to `oui_24_prefix`), days since 1998-01-01
    #[cfg(feature = "regdate")]
    pub(crate) oui_24_regdate: &'static [u16],

    /// MA-M (28-bit prefix) subtable descriptors + prefix/loc memory pools
    pub(crate) oui_28: &'static [SubtableDesc],
    pub(crate) prefix_pool_28: &'static [u8],
    pub(crate) loc_pool_28: &'static [OuiLoc],
    #[cfg(feature = "regdate")]
    pub(crate) regdate_pool_28: &'static [u16],

    /// MA-S (36-bit prefix) subtable descriptors + prefix/loc memory pools
    pub(crate) oui_36: &'static [SubtableDesc],
    pub(crate) prefix_pool_36: &'static [u16],
    pub(crate) loc_pool_36: &'static [OuiLoc],
    #[cfg(feature = "regdate")]
    pub(crate) regdate_pool_36: &'static [u16],
}

impl core::fmt::Debug for OuiDbBundled {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OuiDbBundled")
            .field("updated", &self.updated)
            .field("names_len", &self.names.len())
            .field("oui_24_entries", &self.oui_24_loc.len())
            .field("oui_28_tables", &self.oui_28.len())
            .field("oui_36_tables", &self.oui_36.len())
            .finish()
    }
}

impl super::sealed::Sealed for OuiDbBundled {}

impl OuiBackend for OuiDbBundled {
    fn names(&self) -> &str {
        self.names
    }

    fn oui_24_prefix(&self) -> &[[u8; 3]] {
        self.oui_24_prefix
    }

    fn oui_24_loc(&self) -> &[OuiLoc] {
        self.oui_24_loc
    }

    fn sub28(&self, i: usize) -> (&[u8], &[OuiLoc]) {
        let d = &self.oui_28[i];
        (
            d.prefix_slice(self.prefix_pool_28),
            d.loc_slice(self.loc_pool_28),
        )
    }

    fn sub36(&self, i: usize) -> (&[u16], &[OuiLoc]) {
        let d = &self.oui_36[i];
        (
            d.prefix_slice(self.prefix_pool_36),
            d.loc_slice(self.loc_pool_36),
        )
    }

    #[cfg(feature = "regdate")]
    fn regdate_24(&self, idx: usize) -> u16 {
        self.oui_24_regdate[idx]
    }

    #[cfg(feature = "regdate")]
    fn regdate_28(&self, sub_idx: usize, i: usize) -> u16 {
        let d = &self.oui_28[sub_idx];
        self.regdate_pool_28[d.loc_start as usize + i]
    }

    #[cfg(feature = "regdate")]
    fn regdate_36(&self, sub_idx: usize, i: usize) -> u16 {
        let d = &self.oui_36[sub_idx];
        self.regdate_pool_36[d.loc_start as usize + i]
    }

    fn entry_source(&self) -> EntrySource<'_> {
        EntrySource::Bundled(self)
    }
}

impl OuiDbBundled {
    /// Looks up the organization entry for a MAC address given as a byte slice.
    ///
    /// At least 3 bytes are required (only the leading bytes are significant),
    /// so this accepts both 6-byte MAC and 8-byte EUI-64 addresses. Supports
    /// 24-bit, 28-bit, and 36-bit OUI queries. See [`OuiDb::lookup`](super::OuiDb::lookup)
    /// for the "IEEE Registration Authority" fallback semantics.
    pub fn lookup(&self, mac: &[u8]) -> Option<OuiEntry<'_>> {
        lookup_impl(self, mac)
    }

    /// Looks up the organization entry for a MAC/EUI address given as a string.
    ///
    /// Convenience wrapper over [`OuiDb::lookup`] that accepts human-formatted
    /// input. The separators `.`, `-` and `:` are ignored anywhere in the
    /// string, so all of these are equivalent:
    ///
    /// - `"00:00:0C:00:00:00"`
    /// - `"00-00-0C-00-00-00"`
    /// - `"0000.0C00.0000"` (Cisco dotted)
    /// - `"00000C000000"` (continuous hex)
    ///
    /// **Partial addresses are supported.** An odd number of hex digits is
    /// allowed; a trailing lone digit is treated as the high nibble of a final
    /// byte (low nibble zero). For example `"B8:4C:87:9"` decodes to
    /// `[0xB8, 0x4C, 0x87, 0x90]`, which lets callers probe a bare 28-bit
    /// sub-prefix. At least 3 bytes must be present; the same "IEEE Registration
    /// Authority" fallback as [`OuiDb::lookup`] applies.
    ///
    /// Returns `None` if the string contains a non-hex character or fewer than 3
    /// significant hex digits.
    ///
    pub fn lookup_str(&self, mac: &str) -> Option<OuiEntry<'_>> {
        let bytes = super::parse_partial_hex(mac)?;
        self.lookup(&bytes)
    }

    /// Looks up the organization name (OUI vendor information) for a given MAC address.
    pub fn lookup_mac(&self, mac: crate::MacAddress) -> Option<OuiEntry<'_>> {
        self.lookup(&mac.octets())
    }

    /// Looks up the organization name for an **8-byte** (EUI-64) MAC address.
    ///
    /// Only the leading bytes are significant; behaves like [`lookup`](Self::lookup).
    pub fn lookup_mac6(&self, mac: crate::MacAddress8) -> Option<OuiEntry<'_>> {
        self.lookup(&mac.octets())
    }

    /// Returns an iterator over every [`OuiEntry`](super::OuiEntry) in the database.
    pub fn iter(&self) -> OuiIter<'_> {
        OuiIter {
            inner: OuiIterInner::Bundled(OuiTableIter::new(self)),
        }
    }

    /// Calls the handler `f` for every [`OuiEntry`](super::OuiEntry) in the database.
    pub fn for_each<'a, F>(&'a self, mut f: F)
    where
        F: FnMut(super::OuiEntry<'a>),
    {
        for entry in self.iter() {
            f(entry);
        }
    }

    /// Data source update time (from the `# Updated:` header comment in the data file)
    pub fn updated(&self) -> &str {
        self.updated
    }

    /// Returns the total size in bytes of the embedded static data referenced by this database.
    ///
    /// This sums the organization-name string, every lookup-table entry (the
    /// `prefix` and `loc` arrays), and both the subtable descriptors and their
    /// contents. It is useful for reporting the memory footprint of the OUI
    /// database (e.g. in documentation or diagnostics).
    pub fn data_size_bytes(&self) -> usize {
        let mut total = self.names.len();
        total += core::mem::size_of_val(self.oui_24_prefix);
        total += core::mem::size_of_val(self.oui_24_loc);
        #[cfg(feature = "regdate")]
        {
            total += core::mem::size_of_val(self.oui_24_regdate);
        }
        total += core::mem::size_of_val(self.oui_28);
        total += core::mem::size_of_val(self.prefix_pool_28);
        total += core::mem::size_of_val(self.loc_pool_28);
        total += core::mem::size_of_val(self.oui_36);
        total += core::mem::size_of_val(self.prefix_pool_36);
        total += core::mem::size_of_val(self.loc_pool_36);
        #[cfg(feature = "regdate")]
        {
            total += core::mem::size_of_val(self.regdate_pool_28);
            total += core::mem::size_of_val(self.regdate_pool_36);
        }
        total
    }
}
