//! MAC address OUI lookup command-line tool
//!
//! Usage: `macaddr-oui <MAC address>`
//!
//! Examples:
//!   macaddr-oui 00:55:DA:0A:BB:CC
//!   macaddr-oui 00-55-DA-0A-BB-CC

use macaddr_ouidb::{MacAddress, OuiDb, oui_db};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <MAC address>", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} 00:55:DA:0A:BB:CC", args[0]);
        eprintln!("  {} 00-55-DA-0A-BB-CC", args[0]);
        eprintln!();
        eprintln!("MAC address format:");
        eprintln!("  - Colon-separated: 00:11:22:33:44:55");
        eprintln!("  - Dash-separated:  00-11-22-33-44-55");
        process::exit(1);
    }

    let mac_str = &args[1];

    // Parse the MAC address
    let mac: MacAddress = match mac_str.parse() {
        Ok(mac) => mac,
        Err(e) => {
            eprintln!("Error: Invalid MAC address '{}'", mac_str);
            eprintln!("Details: {:?}", e);
            process::exit(1);
        }
    };

    // Print MAC address information
    println!("MAC Address: {}", mac);
    println!("Formatted:   {}", mac);
    println!(
        "Octets:      {:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}",
        mac.octets()[0],
        mac.octets()[1],
        mac.octets()[2],
        mac.octets()[3],
        mac.octets()[4],
        mac.octets()[5]
    );
    println!();

    // Address type
    println!("Address Type:");
    println!(
        "  Unicast:     {}",
        if mac.is_unicast() { "Yes" } else { "No" }
    );
    println!(
        "  Multicast:   {}",
        if mac.is_multicast() { "Yes" } else { "No" }
    );
    println!(
        "  Broadcast:   {}",
        if mac.is_broadcast() { "Yes" } else { "No" }
    );
    println!(
        "  Universal:   {}",
        if mac.is_universal() { "Yes" } else { "No" }
    );
    println!(
        "  Local:       {}",
        if mac.is_local() { "Yes" } else { "No" }
    );
    println!();

    // OUI lookup
    let db = oui_db();
    match db.lookup_mac(mac) {
        Some(entry) => {
            println!(
                "Organization: {} ({})",
                entry.name(),
                entry.country_or_region_code()
            );
            println!("OUI prefix:   {}", entry.prefix_hex());
            #[cfg(feature = "address")]
            {
                let address = entry.address();
                if !address.is_empty() {
                    println!("Address:     {address}");
                }
            }
            if OuiDb::is_virtual_nic(entry.name()) {
                println!("  ⚠️  This is a virtual NIC");
            }
        }
        None => {
            println!("Organization: Unknown (not found in OUI database)");
        }
    }
}
