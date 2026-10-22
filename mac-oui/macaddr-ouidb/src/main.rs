//! MAC address OUI lookup command-line tool
//!
//! Usage: `macaddr-oui <MAC address>`
//!
//! Examples:
//!   macaddr-oui 00:55:DA:0A:BB:CC
//!   macaddr-oui 00-55-DA-0A-BB-CC
//!   macaddr-oui 00:55:DA:0A:BB:CC:0D:3A
//!   macaddr-oui 0055.DA0A.BBCC.0D3A
//!   macaddr-oui B8:4C:87:9        (partial prefix; trailing half-byte allowed)

use macaddr_ouidb::{MacAddress, MacAddress8, OuiDb, oui_db};
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
        eprintln!("  {} 00:55:DA:0A:BB:CC:0D:3A", args[0]);
        eprintln!("  {} 0055.DA0A.BBCC.0D3A", args[0]);
        eprintln!();
        eprintln!("MAC address format:");
        eprintln!("  - Separators . - : are ignored anywhere");
        eprintln!("  - A trailing lone hex digit is a half-byte (high nibble)");
        eprintln!("  - At least 3 bytes / 6 hex digits are required");
        eprintln!("  - 6 bytes (EUI-48): colon-separated 00:11:22:33:44:55");
        eprintln!("  - 6 bytes (EUI-48): dash-separated  00-11-22-33-44-55");
        eprintln!("  - 8 bytes (EUI-64): colon-separated 00:11:22:33:44:55:66:77");
        eprintln!("  - 8 bytes (EUI-64): Cisco dotted   0011.2233.4455.6677");
        process::exit(1);
    }

    let mac_str = &args[1];

    // Try an 8-byte (EUI-64) address first, then fall back to 6-byte (EUI-48).
    let parsed: Option<([u8; 8], usize)> = if let Ok(mac8) = mac_str.parse::<MacAddress8>() {
        Some((mac8.octets(), 8))
    } else {
        mac_str.parse::<MacAddress>().ok().map(|mac6| {
            let mut o = [0u8; 8];
            o[..6].copy_from_slice(&mac6.octets());
            (o, 6)
        })
    };

    let db = oui_db();

    // Partial input (e.g. a bare 28-bit sub-prefix) has no full address form, so
    // parse it fails and we fall back to a string lookup directly.
    let mac: &[u8] = match &parsed {
        Some((octets, len)) => &octets[..*len],
        None => match db.lookup_str(mac_str) {
            Some(entry) => {
                println!("MAC Address: {} (partial)", mac_str);
                println!("OUI prefix:  {}", entry.prefix_hex());
                println!(
                    "Organization: {} ({})",
                    entry.name(),
                    entry.country_or_region_code()
                );
                return;
            }
            None => {
                eprintln!("Error: Invalid MAC address '{}'", mac_str);
                process::exit(1);
            }
        },
    };

    // Print MAC address information
    println!("MAC Address: {}", mac_str);
    print!("Octets:      ");
    for (i, b) in mac.iter().enumerate() {
        if i > 0 {
            print!("-");
        }
        print!("{b:02X}");
    }
    println!();
    println!();

    // Address type
    println!("Address Type:");
    println!(
        "  Unicast:     {}",
        if mac[0] & 0x01 == 0 { "Yes" } else { "No" }
    );
    println!(
        "  Multicast:   {}",
        if mac[0] & 0x01 != 0 { "Yes" } else { "No" }
    );
    println!(
        "  Broadcast:   {}",
        if mac.iter().all(|&b| b == 0xFF) {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "  Universal:   {}",
        if mac[0] & 0x02 == 0 { "Yes" } else { "No" }
    );
    println!(
        "  Local:       {}",
        if mac[0] & 0x02 != 0 { "Yes" } else { "No" }
    );
    println!();

    // OUI lookup
    match db.lookup(mac) {
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
            #[cfg(feature = "regdate")]
            {
                let date = entry.registered_date();
                if !date.is_empty() {
                    println!("Registered:  {date}");
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
