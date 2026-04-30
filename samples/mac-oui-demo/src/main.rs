//! mac-oui-demo - Comprehensive demonstration of macaddr-ouidb features
//!
//! This demo showcases:
//! - MAC address parsing and validation
//! - Address type detection (unicast/multicast/broadcast/etc.)
//! - OUI lookup (24/28/36-bit)
//! - Virtual NIC detection
//! - Serde serialization
//! - pnet interop
//! - Batch processing

use macaddr_ouidb::{MacAddress, OUI_DB, OuiDb};
use pnet_base::MacAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct NetworkDevice {
    mac: MacAddress,
    name: String,
    manufacturer: Option<String>,
    is_virtual: bool,
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       macaddr-ouidb - Feature Demonstration              ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    demo_basic_operations();
    demo_oui_lookup();
    demo_virtual_detection();
    demo_special_addresses();
    demo_serde();
    demo_pnet_interop();
    demo_batch_processing();

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                  Demo Complete! ✓                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
}

fn demo_basic_operations() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 1. Basic MAC Address Operations                          │");
    println!("└───────────────────────────────────────────────────────────┘");

    let test_cases = vec![
        ("Colon-separated", "00:11:22:33:44:55"),
        ("Dash-separated", "00-11-22-33-44-55"),
        ("Uppercase", "AA:BB:CC:DD:EE:FF"),
        ("Lowercase", "aa:bb:cc:dd:ee:ff"),
        ("Mixed case", "Aa:Bb:Cc:Dd:Ee:Ff"),
    ];

    for (desc, mac_str) in test_cases {
        match mac_str.parse::<MacAddress>() {
            Ok(mac) => println!("  {:20} {:17} ✓", desc, mac),
            Err(e) => println!("  {:20} Error: {:?}", desc, e),
        }
    }

    // Construct from bytes
    let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    println!("\n  From bytes: {}", mac);
    println!();
}

fn demo_oui_lookup() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 2. OUI Lookup (24/28/36-bit)                             │");
    println!("└───────────────────────────────────────────────────────────┘");

    let test_cases = vec![
        ("Cisco (24-bit)", "00:00:0C:00:00:00"),
        ("Microsoft (28-bit)", "00:03:FF:00:00:00"),
        ("Apple (36-bit)", "00:03:93:FF:FF:FF"),
        ("Dell", "00:14:22:00:00:00"),
        ("HP", "00:1A:4B:00:00:00"),
        ("Intel", "00:1E:C9:00:00:00"),
        ("Unknown", "FF:FF:FF:00:00:00"),
    ];

    for (desc, mac_str) in test_cases {
        let mac = mac_str.parse::<MacAddress>().unwrap();
        match OUI_DB.lookup_mac(mac) {
            Some(org) => {
                println!("  {:25} {}", desc, org);
                println!("    MAC: {}", mac);
            }
            None => println!("  {:25} Not found in database", desc),
        }
    }
    println!();
}

fn demo_virtual_detection() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 3. Virtual NIC Detection                                 │");
    println!("└───────────────────────────────────────────────────────────┘");

    let vms = vec![
        ("QEMU", "52:54:00:12:34:56"),
        ("VirtualBox", "08:00:27:00:00:00"),
        ("VMware", "00:0C:29:00:00:00"),
        ("Hyper-V", "00:15:5D:00:00:00"),
        ("Physical (Dell)", "00:14:22:01:02:03"),
    ];

    for (desc, mac_str) in vms {
        let mac = mac_str.parse::<MacAddress>().unwrap();
        if let Some(org) = OUI_DB.lookup_mac(mac) {
            let is_virtual = OuiDb::is_virtual_nic(org);
            let icon = if is_virtual { "🖥️ " } else { "💻 " };
            println!("  {}{:20} {}", icon, desc, org);
            println!("    Virtual: {}, MAC: {}", is_virtual, mac);
        }
    }
    println!();
}

fn demo_special_addresses() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 4. Special MAC Addresses                                 │");
    println!("└───────────────────────────────────────────────────────────┘");

    let zero = MacAddress::zero();
    let broadcast = MacAddress::broadcast();
    let unicast = MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let multicast = MacAddress::new([0x01, 0x00, 0x5E, 0x00, 0x00, 0x01]);
    let local = MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let universal = MacAddress::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    println!("  Zero Address:");
    println!("    MAC: {}", zero);
    println!("    is_zero: {}, is_broadcast: {}", zero.is_zero(), zero.is_broadcast());

    println!("\n  Broadcast Address:");
    println!("    MAC: {}", broadcast);
    println!("    is_broadcast: {}, is_multicast: {}", broadcast.is_broadcast(), broadcast.is_multicast());

    println!("\n  Unicast Address:");
    println!("    MAC: {}", unicast);
    println!("    is_unicast: {}, is_multicast: {}", unicast.is_unicast(), unicast.is_multicast());

    println!("\n  Multicast Address:");
    println!("    MAC: {}", multicast);
    println!("    is_multicast: {}, is_unicast: {}", multicast.is_multicast(), multicast.is_unicast());

    println!("\n  Local Address (LAA):");
    println!("    MAC: {}", local);
    println!("    is_local: {}, is_universal: {}", local.is_local(), local.is_universal());

    println!("\n  Universal Address (UAA):");
    println!("    MAC: {}", universal);
    println!("    is_universal: {}, is_local: {}", universal.is_universal(), universal.is_local());
    println!();
}

fn demo_serde() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 5. Serde Serialization                                   │");
    println!("└───────────────────────────────────────────────────────────┘");

    let devices = vec![
        NetworkDevice {
            mac: MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            name: "Ethernet Adapter".to_string(),
            manufacturer: Some("Intel".to_string()),
            is_virtual: false,
        },
        NetworkDevice {
            mac: MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            name: "Virtual NIC".to_string(),
            manufacturer: Some("QEMU".to_string()),
            is_virtual: true,
        },
    ];

    println!("  Original devices:");
    for dev in &devices {
        println!("    - {} @ {}", dev.name, dev.mac);
    }

    // Serialize
    let json = serde_json::to_string_pretty(&devices).unwrap();
    println!("\n  Serialized JSON:");
    for line in json.lines() {
        println!("    {}", line);
    }

    // Deserialize
    let loaded: Vec<NetworkDevice> = serde_json::from_str(&json).unwrap();
    println!("\n  Deserialized: {} devices", loaded.len());
    println!();
}

fn demo_pnet_interop() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 6. pnet Interop                                          │");
    println!("└───────────────────────────────────────────────────────────┘");

    let pnet_mac = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
    println!("  Original pnet MacAddr:");
    println!("    {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        pnet_mac.octets()[0], pnet_mac.octets()[1], pnet_mac.octets()[2],
        pnet_mac.octets()[3], pnet_mac.octets()[4], pnet_mac.octets()[5]);

    // Convert to macaddr-ouidb
    let mac: MacAddress = pnet_mac.into();
    println!("\n  Converted to MacAddress:");
    println!("    {}", mac);

    // Convert back
    let octets: [u8; 6] = mac.octets().try_into().unwrap();
    let pnet_back = MacAddr::from(octets);
    println!("\n  Converted back to MacAddr:");
    println!("    {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        pnet_back.octets()[0], pnet_back.octets()[1], pnet_back.octets()[2],
        pnet_back.octets()[3], pnet_back.octets()[4], pnet_back.octets()[5]);
    println!();
}

fn demo_batch_processing() {
    println!("┌───────────────────────────────────────────────────────────┐");
    println!("│ 7. Batch Processing                                      │");
    println!("└───────────────────────────────────────────────────────────┘");

    let mac_range: Vec<&str> = (0..5)
        .map(|i| format!("00:11:22:33:44:{:02X}", i))
        .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
        .collect();

    println!("  Processing {} MAC addresses...", mac_range.len());

    let mut stats = BatchStats::new();

    for mac_str in mac_range {
        if let Ok(mac) = mac_str.parse::<MacAddress>() {
            stats.total += 1;
            
            if mac.is_unicast() { stats.unicast += 1; }
            if mac.is_multicast() { stats.multicast += 1; }
            if mac.is_local() { stats.local += 1; }
            if mac.is_universal() { stats.universal += 1; }

            if let Some(org) = OUI_DB.lookup_mac(mac) {
                stats.with_oui += 1;
                if OuiDb::is_virtual_nic(org) {
                    stats.is_virtual += 1;
                }
            }
        }
    }

    println!("\n  Statistics:");
    println!("    Total processed: {}", stats.total);
    println!("    Unicast: {}", stats.unicast);
    println!("    Multicast: {}", stats.multicast);
    println!("    Local (LAA): {}", stats.local);
    println!("    Universal (UAA): {}", stats.universal);
    println!("    With OUI info: {}", stats.with_oui);
    println!("    Virtual NICs: {}", stats.is_virtual);
    println!();
}

#[derive(Debug, Default)]
struct BatchStats {
    total: u32,
    unicast: u32,
    multicast: u32,
    local: u32,
    universal: u32,
    with_oui: u32,
    is_virtual: u32,
}

impl BatchStats {
    fn new() -> Self {
        Self::default()
    }
}
