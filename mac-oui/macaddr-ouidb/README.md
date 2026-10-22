# macaddr-ouidb

[:us: English](README.md) | [:cn: 中文](README-cn.md)

High-performance MAC address OUI (Organizationally Unique Identifier) lookup library, supporting manufacturer information lookup from MAC addresses.

## Features

- 🚀 **Zero-Cost Abstraction**: Compile-time generated lookup tables, only binary search at runtime
- 💾 **Memory Efficient**: Columnar storage + compact encoding
- ⚡ **Fast Lookup**: O(log n) time complexity, supports 24/28/36-bit OUI
- 🔍 **Comprehensive Coverage**: Contains 40,000+ OUI-24 entries, 6,600+ OUI-28 (MA-M) entries, 11,000+ OUI-36 (MA-S/IAB) entries
- 🌍 **Region Info**: Each entry carries an ISO 3166-1 alpha-2 country code alongside the organization name
- 🏢 **Registered Address (optional)**: With the `address` feature each entry also exposes the registered organization address via `OuiEntry::address()`
- 📅 **Registered Date (optional)**: With the `regdate` feature each entry also exposes the registration date (`"YYYY-MM-DD"`) via `OuiEntry::registered_date()`
- 🎯 **Virtual NIC Detection**: Built-in common virtualization platform NIC recognition (VMware, QEMU, VirtualBox, OpenStack, etc.)
- 📦 **Flexible Integration**: Supports `serde` serialization, optional `pnet` interoperability
- 🔒 **no_std Support**: Works in `no_std` environments (optional features for std)
- 🔢 **Multiple MAC Address Formats**: Supports parsing MAC addresses in various formats:
  - Colon-separated: `00:11:22:33:44:55`
  - Dash-separated: `00-11-22-33-44-55`
  - Cisco IOS format: `001A.2B3C.4D5E`
  - Continuous hex: `001A2B3C4D5E`

## Quick Start

### Basic Usage

```rust
use macaddr_ouidb::{MacAddress, oui_db};
use std::str::FromStr;

// Create a MAC address
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC").unwrap();

// Lookup manufacturer information (name only)
match mac.oui() {
    Some(org_name) => println!("Manufacturer: {}", org_name),
    None => println!("OUI information not found"),
}

// Lookup the full OUI entry (name, region, prefix)
let db = oui_db();
if let Some(entry) = db.lookup_mac(mac) {
    println!("Manufacturer: {} ({})", entry.name(), entry.country_or_region_code());
    println!("OUI prefix: {}", entry.prefix_hex());
}

// Check if virtual NIC
if mac.is_virtual_nic() {
    println!("This is a virtual NIC: {}", mac.oui().unwrap_or_else(|| "Unknown".to_owned()));
}

// Check address type
println!("Unicast: {}", mac.is_unicast());
println!("Universal: {}", mac.is_universal());
```

### Registered Address (optional)

With the `address` feature the bundled dataset additionally stores each
organization's registered address. `OuiEntry::address()` / `OuiEntryOwned::address()`
return it; the organization `name()` stays address-free. Entries without an address
(e.g. custom entries, or a runtime database loaded from a file without addresses)
return `""`:

```rust,ignore
use macaddr_ouidb::oui_db;

let db = oui_db();
let mac = "00:00:01:00:00:00".parse().unwrap();
if let Some(entry) = db.lookup_mac(mac) {
    println!("Organization: {}", entry.name());
    println!("Address:      {}", entry.address());
}
```

### Registered Date (optional)

With the `regdate` feature the bundled dataset additionally stores each
organization's registration (assignment) date. `OuiEntry::registered_date()` /
`OuiEntryOwned::registered_date()` return it as an ISO `"YYYY-MM-DD"` string;
entries with an unknown date return `""`. Dates are stored internally as the
number of days since 1998-01-01 (a `u16`), so the feature adds roughly 2 bytes
per lookup entry. Lines without a date in the source data fall back to the
file's `# Updated:` date; a prefix that only has subtables uses the earliest
date among its subtable entries.

```rust,ignore
use macaddr_ouidb::oui_db;

let db = oui_db();
let mac = "00:00:01:00:00:00".parse().unwrap();
if let Some(entry) = db.lookup_mac(mac) {
    println!("Organization: {}", entry.name());
    println!("Registered:   {}", entry.registered_date());
}
```

### Serde Support

After enabling the `serde` feature, JSON serialization is supported:

```rust,ignore
use macaddr_ouidb::MacAddress;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Device {
    mac: MacAddress,
    name: String,
}
```

### Global Database & Runtime Loading

The global accessor `oui_db()` defaults to the compile-time bundled database
(`OUI_DB_BUNDLED`). With the `std` feature you can load a database from data files at
runtime (`OuiDbOwned`) and install it process-wide. `set_oui_db` may be called multiple
times; the latest installation takes effect. The global is backed by an `arc-swap`, so
concurrently held handles (`OuiDbRef`) keep the database they came from alive, and
replaced instances are reclaimed automatically once the last reference is dropped
(no leak). `reset_oui_db` restores the bundled database:

```rust,no_run
# #[cfg(feature = "std")]
# {
use macaddr_ouidb::{set_oui_db, reset_oui_db, oui_db, OuiDb, OuiDbOwned, OUI_DB_BUNDLED};

// Load and install a runtime database (merged from two files).
// `custom-mac-oui.txt` entries are merged over `ieee-mac-oui.txt`.
// The ieee file must carry a `# Updated:` header line.
let owned = OuiDbOwned::from_files("custom-mac-oui.txt", "ieee-mac-oui.txt").unwrap();
set_oui_db(owned.into());

// All lookups now route to the installed database.
assert!(matches!(&*oui_db(), OuiDb::Owned(_)));

// Can be replaced again, e.g. on a hot reload; the latest one wins.
set_oui_db(OuiDb::Bundled(OUI_DB_BUNDLED));
assert!(matches!(&*oui_db(), OuiDb::Bundled(_)));

// Reset back to the bundled database.
reset_oui_db();
assert!(matches!(&*oui_db(), OuiDb::Bundled(_)));
# }
```

Borrowed lookups (`lookup` / `lookup_mac` / `lookup_mac6`) return an `OuiEntry<'_>`,
which is valid only while the database is borrowed — the zero-copy choice for
immediate use. When an entry must outlive the handle (stored in a struct, returned
from a function, or used after `set_oui_db` swaps the database), use the
owned-returning `lookup_owned` / `lookup_mac_owned` / `lookup_mac6_owned`, or
`MacAddress::oui_entry()`: they return an `OuiEntryOwned` that owns its data source
and supports `Clone`, `Debug` and `Display`.

`OuiDb::lookup` accepts any byte slice with **at least 3 bytes** (only the leading
bytes are significant), so it handles both 6-byte `MacAddress` and 8-byte
`MacAddress8` (EUI-64) inputs. Convenience wrappers `lookup_mac` (6-byte) and
`lookup_mac6` (8-byte) take the address types directly. If the leading 3 bytes match
an "IEEE Registration Authority" subtable-parent prefix but the slice is too short
to resolve the subtable (28-bit needs byte 4, 36-bit needs bytes 4-5), a synthetic
entry named `"IEEE Registration Authority"` (country/region `US`, address
`445 Hoes Lane Piscataway NJ US 08554` with the `address` feature) is returned for
that 24-bit prefix.

Without the `std` feature, only the bundled database is available and
`set_oui_db` / `OuiDbOwned` are not compiled.

## API Documentation

### `MacAddress`

Ethernet MAC address type (6 bytes).

```rust
use macaddr_ouidb::{MacAddress, oui_db};
use std::str::FromStr;

// Constructors
let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
let mac = MacAddress::new6(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
let mac: MacAddress = "00-11-22-33-44-55".parse().unwrap();
let bytes = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
let mac: MacAddress = "001A2B3C4D5E".parse().unwrap(); // Continuous hex
let zero = MacAddress::zero();
let broadcast = MacAddress::broadcast();

// Property checks
let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
assert!(mac.is_unicast());
assert!(!mac.is_multicast());
assert!(!mac.is_broadcast());
assert!(mac.is_universal());
assert!(!mac.is_local());
assert!(!mac.is_zero());

// Methods
let octets = mac.octets();           // [u8; 6]
let value = mac.to_u64();            // u64
let oui = mac.oui();                 // Option<String>  (name only)
let db = oui_db();
let entry = db.lookup_mac(mac);        // Option<OuiEntry<'_>>   (borrowed entry)
let entry = db.lookup_mac_owned(mac);  // Option<OuiEntryOwned>  (owned entry)
let entry = mac.oui_entry();           // Option<OuiEntryOwned>  (owned entry)
let is_virtual = mac.is_virtual_nic(); // bool
```

### `MacAddress8`

64-bit EUI-64 address type (8 bytes). OUI lookup only depends on the leading
bytes, so it shares the same database and rules as `MacAddress`.

```rust
use macaddr_ouidb::{MacAddress8, oui_db};

// Constructors
let mac = MacAddress8::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
let mac = MacAddress8::new8(0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77);
let mac: MacAddress8 = "00:11:22:33:44:55:66:77".parse().unwrap();
let mac: MacAddress8 = "00-11-22-33-44-55-66-77".parse().unwrap();
let mac: MacAddress8 = "0011223344556677".parse().unwrap();       // Continuous hex
let mac: MacAddress8 = "0011.2233.4455.6677".parse().unwrap();    // Cisco dotted

// Methods
let octets = mac.octets();             // [u8; 8]
let value = mac.to_u64();              // u64  (big-endian)
let db = oui_db();
let entry = db.lookup_mac6(mac);        // Option<OuiEntry<'_>>   (borrowed entry)
let entry = db.lookup_mac6_owned(mac);  // Option<OuiEntryOwned>  (owned entry)
let entry = mac.oui_entry();            // Option<OuiEntryOwned>  (owned entry)
let is_virtual = mac.is_virtual_nic();  // bool
```

When you already have raw bytes, call `db.lookup(&octets)` directly — it accepts
slices of 3 bytes or more.

## Data Format

Supports three OUI types:

| Type          | Bits | Format          | Description                                           |
| ------------- | ---- | --------------- | ----------------------------------------------------- |
| OUI-24 (MA-L) | 24   | `XX:XX:XX`      | Legacy assignment, first 3 bytes                      |
| OUI-28 (MA-M) | 28   | `XX:XX:XX:X`    | Medium-sized, first 3 bytes + high 4 bits of 4th byte |
| OUI-36 (MA-S) | 36   | `XX:XX:XX:XX:X` | Small-sized, first 4 bytes + high 4 bits of 5th byte  |

Automatically matches the most precise type during lookup.

## Data Source

OUI data is derived from the IEEE official registries (MA-L, MA-M, MA-S/IAB) and
generated at compile time by the [`ouidb-builder`](../ouidb-builder) tool into
`ieee-mac-oui.txt`. A small hand-maintained `custom-mac-oui.txt` supplements the
IEEE data with virtual NIC prefixes (QEMU, Bochs, OpenStack, etc.).

`build.rs` merges both sources at compile time:

- Organization names are stored as `<2-letter region code><name>` (e.g. `JPSony`);
  the country code is extracted from the IEEE organization address.
- Invisible Unicode artifacts present in the vendor-submitted IEEE data (NBSP,
  zero-width spaces, soft hyphens, …) are stripped during generation.
- Duplicate prefixes caused by IEEE reassignments are de-duplicated, keeping the
  **most recent** allocation (see `ouidb-builder/duplicate-prefix.log`).
- If a prefix exists in **both** `custom-mac-oui.txt` and the IEEE data, the build
  fails with an error instead of silently overriding.

Data is automatically generated at compile time via `build.rs`, ensuring:
- Consistency with source code version
- No runtime loading required
- Zero initialization overhead

## Features

| Feature    | Description                                         | Default |
| ---------- | --------------------------------------------------- | ------- |
| `std`      | Enable std library support                          | Yes     |
| `serde`    | Enable serde serialization                          | Yes     |
| `pnet`     | Enable pnet interoperability                        | Yes     |
| `fullname` | Use the full (untrimmed) organization names dataset | No      |
| `address`  | Include registered organization addresses           | No      |
| `regdate`  | Include organization registration dates             | No      |

## no_std Support

This crate supports `no_std` environments. To use it in a `no_std` environment, disable default features:

```toml
[dependencies]
macaddr-ouidb = { version = "0.2", default-features = false }
```

## CLI Tool

The crate includes a CLI tool `mac-oui` for querying MAC address information:

```console
$ cargo run --bin mac-oui -- 00:55:DA:0A:BB:CC
MAC Address: 00:55:da:0a:bb:cc
Formatted:   00:55:da:0a:bb:cc
Octets:      00-55-DA-0A-BB-CC

Address Type:
  Unicast:     Yes
  Multicast:   No
  Broadcast:   No
  Universal:   Yes
  Local:       No

Organization: Shinko Technos (JP)
```

With the `address` feature an additional `Address:` line is printed when the
organization has a registered address. Likewise, with the `regdate` feature a
`Registered:` line prints the registration date.

## Performance

- **Lookup Time**: O(log n) binary search
- **Memory Usage**: ~1.0 MB embedded static data (measured in a release build), covering 40,000+ OUI-24, 6,600+ OUI-28, 11,000+ OUI-36 entries
- **Initialization**: Zero runtime initialization (compile-time generated)

