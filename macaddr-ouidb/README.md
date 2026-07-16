# macaddr-ouidb

High-performance MAC address OUI (Organizationally Unique Identifier) lookup library, supporting manufacturer information lookup from MAC addresses.

## Features

- 🚀 **Zero-Cost Abstraction**: Compile-time generated lookup tables, only binary search at runtime
- 💾 **Memory Efficient**: Columnar storage + compact encoding, full database only ~1.7MB
- ⚡ **Fast Lookup**: O(log n) time complexity, supports 24/28/36-bit OUI
- 🔍 **Comprehensive Coverage**: Contains 38,000+ OUI-24 entries, 6,000+ OUI-28 entries, 6,000+ OUI-36 entries (from Nmap project)
- 🎯 **Virtual NIC Detection**: Built-in common virtualization platform NIC recognition (VMware, QEMU, VirtualBox, OpenStack, etc.)
- 📦 **Flexible Integration**: Supports `serde` serialization, optional `pnet` interoperability
- 🔒 **no_std Support**: Works in `no_std` environments (optional features for std)

## Quick Start

### Basic Usage

```rust
use macaddr_ouidb::MacAddress;
use std::str::FromStr;

// Create a MAC address
let mac = MacAddress::from_str("00:55:DA:0A:BB:CC").unwrap();

// Lookup manufacturer information
match mac.oui() {
    Some(org_name) => println!("Manufacturer: {}", org_name),
    None => println!("OUI information not found"),
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
let mac = MacAddress::from_slice(&bytes).unwrap();
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
let oui = mac.oui();                 // Option<&'static str>
let is_virtual = mac.is_virtual_nic(); // bool
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

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `std` | Enable std library support | Yes |
| `serde` | Enable serde serialization | Yes |
| `pnet` | Enable pnet interoperability | Yes |
| `ma-s` | Enable 36-bit OUI (MA-S) support | Yes |

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

Organization: Shinko Technos
```

## Performance

- **Lookup Time**: O(log n) binary search
- **Memory Usage**: ~1.7MB for full database (38,000+ OUI-24, 6,000+ OUI-28, 6,000+ OUI-36 entries)
- **Initialization**: Zero runtime initialization (compile-time generated)

## License

Apache-2.0

## Contributing

Issues and Pull Requests are welcome!