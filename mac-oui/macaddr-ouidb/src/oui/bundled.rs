//! Bundled (compile-time generated) OUI database

use super::{
    EntrySource, OuiBackend, OuiEntry, OuiIter, OuiIterInner, OuiLoc, OuiTableIter, lookup_impl,
};

/// # Organizationally Unique Identifiers (OUI) database (bundled)
///
/// Data is generated at compile time by `build.rs`, with zero runtime
/// initialization cost.
#[derive(Clone, Copy)]
pub struct OuiDbBundled {
    /// Data source update time (from the `# Updated:` header comment in the data file)
    pub updated: &'static str,

    pub(crate) names: &'static str,

    /// MA-L (24-bit prefix), 24 bits ≈ 16.77M, legacy large vendors / carriers
    pub(crate) oui_24: &'static OuiTable<3>,

    /// MA-M (28-bit prefix), 20 bits ≈ 1.04M, mid-size device makers / module vendors
    pub(crate) oui_28: &'static [OuiSubtable<u8>],

    /// MA-S (36-bit prefix), 12 bits = 4096, IoT / small / dedicated hardware
    pub(crate) oui_36: &'static [OuiSubtable<u16>],
}

impl core::fmt::Debug for OuiDbBundled {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OuiDbBundled")
            .field("updated", &self.updated)
            .field("names_len", &self.names.len())
            .field("oui_24_entries", &self.oui_24.loc.len())
            .field("oui_28_tables", &self.oui_28.len())
            .field("oui_36_tables", &self.oui_36.len())
            .finish()
    }
}

/// Columnar storage (saves memory, improves cache utilization)
pub(crate) struct OuiTable<const N: usize> {
    pub(crate) prefix: &'static [[u8; N]],
    pub(crate) loc: &'static [OuiLoc],
}

/// Columnar storage (saves memory, improves cache utilization)
pub(crate) struct OuiSubtable<T: 'static> {
    pub(crate) prefix: &'static [T],
    pub(crate) loc: &'static [OuiLoc],
}

impl super::sealed::Sealed for OuiDbBundled {}

impl OuiBackend for OuiDbBundled {
    fn names(&self) -> &str {
        self.names
    }

    fn oui_24_prefix(&self) -> &[[u8; 3]] {
        self.oui_24.prefix
    }

    fn oui_24_loc(&self) -> &[OuiLoc] {
        self.oui_24.loc
    }

    fn sub28(&self, i: usize) -> (&[u8], &[OuiLoc]) {
        (self.oui_28[i].prefix, self.oui_28[i].loc)
    }

    fn sub36(&self, i: usize) -> (&[u16], &[OuiLoc]) {
        (self.oui_36[i].prefix, self.oui_36[i].loc)
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
}
