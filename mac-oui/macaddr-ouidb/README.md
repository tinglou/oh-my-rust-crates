# macaddr-ouidb

[:us: English](README.md) | [:cn: 中文](README-cn.md)

High-performance MAC address OUI (Organizationally Unique Identifier) lookup library, supporting manufacturer information lookup from MAC addresses.

## Features

- 🚀 **Zero-Cost Abstraction**: Compile-time generated lookup tables, only binary search at runtime
- 💾 **Memory Efficient**: Columnar storage + compact encoding
- ⚡ **Fast Lookup**: O(log n) time complexity, supports 24/28/36-bit OUI
- 🔍 **Comprehensive Coverage**: Contains 40,000+ OUI-24 entries, 6,600+ OUI-28 (MA-M) entries, 11,000+ OUI-36 (MA-S/IAB) entries
- 🌍 **Region Info**: Each entry carries an ISO 3166-1 alpha-2 country code alongside the organization name
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
use macaddr_ouidb::MacAddress;
use std::str::FromStr;

// Create a MAC address
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC").unwrap();

// Lookup manufacturer information (name only)
match mac.oui() {
    Some(org_name) => println!("Manufacturer: {}", org_name),
    None => println!("OUI information not found"),
}

// Lookup manufacturer name together with its region
if let Some((name, region)) = mac.oui_and_region() {
    println!("Manufacturer: {} ({})", name, region);
}

// Check if virtual NIC
if mac.is_virtual_nic() {
    println!("This is a virtual NIC: {}", mac.oui().unwrap_or("Unknown"));
}

// Check address type
println!("Unicast: {}", mac.is_unicast());
println!("Universal: {}", mac.is_universal());
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

## API Documentation

### `MacAddress`

Ethernet MAC address type (6 bytes).

```rust
use macaddr_ouidb::MacAddress;
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
let oui = mac.oui();                 // Option<&'static str>  (name only)
let pair = mac.oui_and_region();     // Option<(&'static str, &'static str)>  (name, region)
let is_virtual = mac.is_virtual_nic(); // bool
```

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

## no_std Support

This crate supports `no_std` environments. To use it in a `no_std` environment, disable default features:

```toml
[dependencies]
macaddr-ouidb = { version = "0.1", default-features = false }
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

## Performance

- **Lookup Time**: O(log n) binary search
- **Memory Usage**: ~1.0 MB embedded static data (measured in a release build), covering 40,000+ OUI-24, 6,600+ OUI-28, 11,000+ OUI-36 entries
- **Initialization**: Zero runtime initialization (compile-time generated)

