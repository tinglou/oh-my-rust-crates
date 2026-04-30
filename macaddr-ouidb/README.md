# macaddr-ouidb

High-performance MAC address OUI (Organizationally Unique Identifier) lookup library, supporting manufacturer information lookup from MAC addresses.

## Features

- 🚀 **Zero-Cost Abstraction**: Compile-time generated lookup tables, only binary search at runtime
- 💾 **Memory Efficient**: Columnar storage + compact encoding, full database only ~900KB
- ⚡ **Fast Lookup**: O(log n) time complexity, supports 24/28/36-bit OUI
- 🔍 **Comprehensive Coverage**: Contains 52,000+ OUI entries (from Nmap project)
- 🎯 **Virtual NIC Detection**: Built-in common virtualization platform NIC recognition
- 📦 **Flexible Integration**: Supports `serde` serialization, optional `pnet` interoperability


## Quick Start

### Basic Usage

```rust ignore
use macaddr_ouidb::{MacAddress, OUI_DB};

// Create a MAC address
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC")?;

// Lookup manufacturer information
match OUI_DB.lookup(mac) {
    Some(org_name) => println!("Manufacturer: {}", org_name),
    None => println!("OUI information not found"),
}
```

### Virtual NIC Detection

```rust ignore
use macaddr_ouidb::OuiDb;

let org_name = OUI_DB.lookup(mac).unwrap_or("Unknown");

if OuiDb::is_virtual_nic(org_name) {
    println!("This is a virtual NIC: {}", org_name);
}
```

### Serde Support

After enabling the `serde` feature, JSON serialization is supported:

```rust ignore
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

#### Constructors

```rust ignore
// From byte array
let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

// From string (supports ':' or '-' delimiters)
let mac: MacAddress = "00:11:22:33:44:55".parse()?;
let mac: MacAddress = "00-11-22-33-44-55".parse()?;

// Special addresses
let zero = MacAddress::zero();
let broadcast = MacAddress::broadcast();
```

#### Property Checks

```rust ignore
mac.is_zero()          // Is all-zero address
mac.is_broadcast()     // Is broadcast address
mac.is_unicast()       // Is unicast address
mac.is_multicast()     // Is multicast address
mac.is_universal()     // Is universally administered address (UAA)
mac.is_local()         // Is locally administered address (LAA)
```

### `OUI_DB`

Pre-compiled OUI database instance.

```rust ignore
use macaddr_ouidb::OUI_DB;

let org_name = OUI_DB.lookup(mac);  // Option<&'static str>
```

### `OuiDb`

OUI database utility methods.

```rust ignore
// Check if it's a virtual NIC
OuiDb::is_virtual_nic("QEMU virtual NIC")  // true

// Get subtable name
OuiDb::oui_subtable_name()  // "Ieee Registration Authority"
```

## Data Format

Supports three OUI types:

| Type | Bits | Format | Description |
|------|------|--------|-------------|
| OUI-24 (MA-L) | 24 | `XX:XX:XX` | Legacy assignment, first 3 bytes |
| OUI-28 (MA-M) | 28 | `XX:XX:XX:X` | Medium-sized, first 3 bytes + high 4 bits of 4th byte |
| OUI-36 (MA-S) | 36 | `XX:XX:XX:XX:X` | Small-sized, first 4 bytes + high 4 bits of 5th byte |

Automatically matches the most precise type during lookup.

## Data Source

OUI data comes from the `nmap-mac-prefixes` file of the [Nmap project](https://nmap.org/), which integrates IEEE official OUI assignment data.

Data is automatically generated at compile time via `build.rs`, ensuring:
- Consistency with source code version
- No runtime loading required
- Zero initialization overhead

## License

Apache-2.0

## Contributing

Issues and Pull Requests are welcome!

