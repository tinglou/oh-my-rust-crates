//! mac-oui-nostd-demo - Demonstrates macaddr-ouidb with no_std library
//!
//! This demo shows:
//! - macaddr-ouidb compiled without std (default-features = false)
//! - Basic MAC address operations (available in no_std)
//! - Optional serde support (works in no_std)
//! - pnet interop (works in no_std)
//! - OUI lookup (requires std feature in macaddr-ouidb)
//! - OUI entry lookup (lookup_mac / prefix_hex) and iter / for_each
//! - EUI-64 (8-byte) addresses via MacAddress8 / lookup_mac6
//!
//! Note: This binary itself uses std for console output,
//! but demonstrates macaddr-ouidb's no_std compatibility.

use macaddr_ouidb::{oui_db, MacAddress, MacAddress8, OuiDb};
use pnet_base::MacAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Device {
    mac: MacAddress,
    name: String,
}

fn main() {
    println!("=== mac-oui-nostd-demo ===");
    println!(
        "macaddr-ouidb built with: default-features = false, features = [\"serde\", \"pnet\"]\n"
    );

    // Demo 1: Basic MAC address operations (no_std compatible)
    println!("1. Basic MAC Address Operations (no_std compatible)");
    println!("   -----------------------------------------------");
    let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    println!("   MAC Address: {}", mac);
    println!("   Is Zero: {}", mac.is_zero());
    println!("   Is Broadcast: {}", mac.is_broadcast());
    println!("   Is Unicast: {}", mac.is_unicast());
    println!("   Is Multicast: {}", mac.is_multicast());
    println!("   Is Local: {}", mac.is_local());
    println!("   Is Universal: {}", mac.is_universal());
    println!();

    // Demo 2: Parse MAC from string (no_std compatible)
    println!("2. Parse MAC from String (no_std compatible)");
    println!("   -----------------------------------------------");
    let mac_addresses = [
        "00:11:22:33:44:55",
        "AA-BB-CC-DD-EE-FF",
        "52:54:00:12:34:56",
    ];
    for mac_str in &mac_addresses {
        match mac_str.parse::<MacAddress>() {
            Ok(mac) => println!("   {} -> {}", mac_str, mac),
            Err(_) => println!("   {} -> Parse error", mac_str),
        }
    }
    println!();

    // Demo 3: OUI Lookup (requires std feature)
    println!("3. OUI Lookup (requires 'std' feature in macaddr-ouidb)");
    println!("   -----------------------------------------------");
    let oui_test_cases = [
        ("00:00:0C:00:00:00", "Cisco Systems"),
        ("52:54:00:12:34:56", "QEMU virtual NIC"),
        ("00:55:DA:0A:BB:CC", "Unknown"),
    ];
    let db = oui_db();
    for (mac_str, _expected) in &oui_test_cases {
        match mac_str.parse::<MacAddress>() {
            Ok(mac) => match db.lookup_mac(mac) {
                Some(entry) => {
                    println!(
                        "   {} -> {} ({})  prefix={}",
                        mac_str,
                        entry.name(),
                        entry.country_or_region_code(),
                        entry.prefix_hex()
                    );
                    if OuiDb::is_virtual_nic(entry.name()) {
                        println!("      ⚠️  Virtual NIC detected");
                    }
                }
                None => println!("   {} -> Unknown (not in database)", mac_str),
            },
            Err(_) => println!("   {} -> Invalid MAC", mac_str),
        }
    }
    println!();

    // Demo 4: New entry/iterator API (no_std + alloc)
    println!("4. OuiEntry / iter / for_each API (no_std + alloc)");
    println!("   -----------------------------------------------");
    let entry_mac = MacAddress::new([0x00, 0x00, 0x0C, 0x00, 0x00, 0x00]);
    match db.lookup_mac(entry_mac) {
        Some(entry) => println!(
            "   lookup_mac(): {} ({})  prefix={}",
            entry.name(),
            entry.country_or_region_code(),
            entry.prefix_hex()
        ),
        None => println!("   lookup_mac(): None"),
    }
    let total = db.iter().count();
    let mut virtual_count = 0usize;
    db.for_each(|e| {
        if OuiDb::is_virtual_nic(e.name()) {
            virtual_count += 1;
        }
    });
    println!("   iter() total entries: {}", total);
    println!("   for_each() virtual-NIC entries: {}", virtual_count);
    println!();

    // Demo 4b: EUI-64 (8-byte) addresses
    println!("4b. EUI-64 (8-byte) addresses (MacAddress8 / lookup_mac6)");
    println!("   -----------------------------------------------");
    let eui64_samples = ["00:00:0C:00:00:00:12:34", "0011.2233.4455.6677"];
    for mac_str in &eui64_samples {
        match mac_str.parse::<MacAddress8>() {
            Ok(mac) => {
                print!("   {} -> {}", mac_str, mac);
                match db.lookup_mac6(mac) {
                    Some(entry) => println!(
                        "  => {} ({})  prefix={}",
                        entry.name(),
                        entry.country_or_region_code(),
                        entry.prefix_hex()
                    ),
                    None => println!("  => Unknown (not in database)"),
                }
            }
            Err(_) => println!("   {} -> Invalid EUI-64", mac_str),
        }
    }
    println!();

    // Demo 5: Serde Serialization (no_std compatible with serde feature)
    println!("5. Serde Serialization (no_std + serde feature)");
    println!("   -----------------------------------------------");
    let device = Device {
        mac: MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
        name: "Test Device".to_string(),
    };
    match serde_json::to_string_pretty(&device) {
        Ok(json) => {
            println!("   Serialized:");
            for line in json.lines() {
                println!("      {}", line);
            }
        }
        Err(e) => println!("   Serialization error: {:?}", e),
    }

    let json_str = r#"{"mac":"00:11:22:33:44:55","name":"Deserialized Device"}"#;
    match serde_json::from_str::<Device>(json_str) {
        Ok(dev) => println!("   Deserialized: {} @ {}", dev.name, dev.mac),
        Err(e) => println!("   Deserialization error: {:?}", e),
    }
    println!();

    // Demo 6: pnet Interop (no_std compatible)
    println!("6. pnet Interop (no_std + pnet feature)");
    println!("   -----------------------------------------------");
    let pnet_mac = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
    println!(
        "   pnet MacAddr: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        pnet_mac.octets()[0],
        pnet_mac.octets()[1],
        pnet_mac.octets()[2],
        pnet_mac.octets()[3],
        pnet_mac.octets()[4],
        pnet_mac.octets()[5]
    );

    // Convert pnet -> macaddr-ouidb
    let mac_from_pnet: MacAddress = pnet_mac.into();
    println!("   Converted to MacAddress: {}", mac_from_pnet);

    // Convert back
    let octets: [u8; 6] = mac_from_pnet.octets();
    let pnet_mac_back = MacAddr::from(octets);
    println!(
        "   Converted back: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        pnet_mac_back.octets()[0],
        pnet_mac_back.octets()[1],
        pnet_mac_back.octets()[2],
        pnet_mac_back.octets()[3],
        pnet_mac_back.octets()[4],
        pnet_mac_back.octets()[5]
    );
    println!();

    println!("=== Demo Complete ===");
}
