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

    /// MA-M (28-bit prefix) subtable descriptors + prefix/loc memory pools
    pub(crate) oui_28: &'static [SubtableDesc],
    pub(crate) prefix_pool_28: &'static [u8],
    pub(crate) loc_pool_28: &'static [OuiLoc],

    /// MA-S (36-bit prefix) subtable descriptors + prefix/loc memory pools
    pub(crate) oui_36: &'static [SubtableDesc],
    pub(crate) prefix_pool_36: &'static [u16],
    pub(crate) loc_pool_36: &'static [OuiLoc],
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

    fn entry_source(&self) -> EntrySource<'_> {
        EntrySource::Bundled(self)
    }
}

impl OuiDbBundled {
    /// Looks up the organization entry for a given MAC address byte array.
    ///
    /// Supports 24-bit, 28-bit, and 36-bit OUI queries.
    pub fn lookup(&self, mac: [u8; 6]) -> Option<OuiEntry<'_>> {
        lookup_impl(self, mac)
    }

    /// Looks up the organization name (OUI vendor information) for a given MAC address.
    pub fn lookup_mac(&self, mac: crate::MacAddress) -> Option<OuiEntry<'_>> {
        self.lookup(mac.octets())
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
        total += core::mem::size_of_val(self.oui_28);
        total += core::mem::size_of_val(self.prefix_pool_28);
        total += core::mem::size_of_val(self.loc_pool_28);
        total += core::mem::size_of_val(self.oui_36);
        total += core::mem::size_of_val(self.prefix_pool_36);
        total += core::mem::size_of_val(self.loc_pool_36);
        total
    }
}
