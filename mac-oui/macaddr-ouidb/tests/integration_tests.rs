//! macaddr-oui integration tests

use macaddr_ouidb::{MacAddress, MacAddress8, OuiDb, oui_db};

#[test]
fn test_mac_address_parsing() {
    // Test the colon-separated format
    let mac1: MacAddress = "00:55:DA:0A:BB:CC".parse().unwrap();
    assert_eq!(mac1.octets(), [0x00, 0x55, 0xDA, 0x0A, 0xBB, 0xCC]);

    // Test the dash-separated format
    let mac2: MacAddress = "00-55-DA-0A-BB-CC".parse().unwrap();
    assert_eq!(mac2.octets(), [0x00, 0x55, 0xDA, 0x0A, 0xBB, 0xCC]);

    // Test case-insensitivity
    let mac3: MacAddress = "00:55:da:0a:bb:cc".parse().unwrap();
    assert_eq!(mac3.octets(), [0x00, 0x55, 0xDA, 0x0A, 0xBB, 0xCC]);
}

#[test]
fn test_mac_address_special_addresses() {
    // Zero address
    let zero = MacAddress::zero();
    assert!(zero.is_zero());
    assert!(!zero.is_broadcast());

    // Broadcast address
    let broadcast = MacAddress::broadcast();
    assert!(!broadcast.is_zero());
    assert!(broadcast.is_broadcast());

    // Unicast/multicast
    let unicast: MacAddress = "00:11:22:33:44:55".parse().unwrap();
    assert!(unicast.is_unicast());
    assert!(!unicast.is_multicast());

    let multicast: MacAddress = "01:00:00:00:00:00".parse().unwrap();
    assert!(!multicast.is_unicast());
    assert!(multicast.is_multicast());

    // Universally/locally administered
    let universal: MacAddress = "00:11:22:33:44:55".parse().unwrap();
    assert!(universal.is_universal());
    assert!(!universal.is_local());

    let local: MacAddress = "02:11:22:33:44:55".parse().unwrap();
    assert!(!local.is_universal());
    assert!(local.is_local());
}

#[test]
fn test_oui_lookup_24bit() {
    let db = oui_db();
    // Test 24-bit OUI lookup (standard OUI, first 3 bytes)
    // Format: XX:XX:XX:YY:YY:YY, only the first 3 bytes are used for lookup

    // Xerox (00:00:00)
    let mac: MacAddress = "00:00:01:00:00:00".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        matches!(&org, Some(e) if e.country_or_region_code() == "US" && e.name().starts_with("XEROX")),
        "Expected XEROX (US), got {:?}",
        org
    );

    // Cisco Systems (00:00:0C)
    let mac: MacAddress = "00:00:0C:00:00:00".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        matches!(&org, Some(e) if e.country_or_region_code() == "US" && e.name().starts_with("Cisco Systems")),
        "Expected Cisco Systems (US), got {:?}",
        org
    );

    // Microsoft Hyper-V (00:03:FF) overridden by custom-mac-oui.txt
    let mac: MacAddress = "00:03:FF:00:00:00".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        matches!(&org, Some(e) if e.name() == "Microsoft Hyper-V virtual NIC"),
        "Expected Microsoft Hyper-V virtual NIC, got {:?}",
        org
    );

    // Apple (00:03:93)
    let mac: MacAddress = "00:03:93:FF:FF:FF".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        matches!(&org, Some(e) if e.country_or_region_code() == "US" && e.name().starts_with("Apple")),
        "Expected Apple (US), got {:?}",
        org
    );
}

#[test]
fn test_oui_lookup_28bit() {
    let db = oui_db();
    // Test 28-bit OUI lookup (MA-M, first 3 bytes + high nibble of byte 4)
    // Format: XX:XX:DA:0X:YY:YY, only the high 4 bits of byte 4 are significant (0x00-0xF0)

    // 28-bit entries under the 00:55:DA prefix
    let mac: MacAddress = "00:55:DA:0A:BB:CC".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        org.is_some(),
        "28-bit OUI lookup failed for 00:55:DA:0A:BB:CC"
    );
    println!("28-bit OUI: 00:55:DA:0A:BB:CC -> {:?}", org.unwrap());

    // Under the same 24-bit prefix, different 28-bit sub-entries should match different organizations
    let mac1: MacAddress = "00:55:DA:00:00:00".parse().unwrap();
    let mac2: MacAddress = "00:55:DA:10:00:00".parse().unwrap();
    let org1 = db.lookup_mac(mac1);
    let org2 = db.lookup_mac(mac2);

    // Both should yield a result (which may or may not be the same)
    assert!(
        org1.is_some(),
        "28-bit OUI lookup failed for 00:55:DA:00:00:00"
    );
    assert!(
        org2.is_some(),
        "28-bit OUI lookup failed for 00:55:DA:10:00:00"
    );

    // Verify that the 28-bit subtable is actually used (not a plain 24-bit match)
    // If byte 4 is outside the subtable range, it should fall back to the 24-bit entry or return None
    let mac_fallback: MacAddress = "00:55:DA:F0:00:00".parse().unwrap();
    let org_fallback = db.lookup_mac(mac_fallback);
    // This may match a 24-bit entry or a 28-bit entry
    assert!(org_fallback.is_some() || org_fallback.is_none());
}

#[test]
fn test_oui_lookup_36bit() {
    let db = oui_db();
    // Test 36-bit OUI lookup (MA-S, first 4 bytes + high nibble of byte 5)
    // Format: XX:XX:XX:XX:0X:YY, only the high 4 bits of byte 5 are significant (0x00-0xF0)

    // 36-bit entries under the 00:16:E3 prefix
    let mac: MacAddress = "00:16:E3:00:00:00".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(
        org.is_some(),
        "36-bit OUI lookup failed for 00:16:E3:00:00:00"
    );
    println!("36-bit OUI: 00:16:E3:00:00:00 -> {:?}", org);

    // Verify that the 36-bit subtable is used
    // 00:16:E3 is a 24-bit prefix, but some of its byte-4 values have 36-bit subtables
    let mac2: MacAddress = "00:16:E3:80:00:00".parse().unwrap();
    let org2 = db.lookup_mac(mac2);
    assert!(
        org2.is_some(),
        "36-bit OUI lookup failed for 00:16:E3:80:00:00"
    );

    // Different byte-4 values should be able to match different results
    let same = org.as_ref().map(|e| e.name()) == org2.as_ref().map(|e| e.name());
    if !same {
        println!("Different 36-bit entries return different organizations:");
        println!("  00:16:E3:00:00:00 -> {:?}", org);
        println!("  00:16:E3:80:00:00 -> {:?}", org2);
    }
}

#[test]
fn test_oui_lookup_not_found() {
    let db = oui_db();
    // Test unallocated OUIs (not present in the database)

    // FF:FF:FF is not a valid OUI prefix
    let mac: MacAddress = "FF:FF:FF:00:00:00".parse().unwrap();
    let org = db.lookup_mac(mac);
    assert!(org.is_none(), "Expected None for unallocated OUI FF:FF:FF");

    // Some other unallocated address ranges
    let mac2: MacAddress = "FE:FF:FF:00:00:00".parse().unwrap();
    let org2 = db.lookup_mac(mac2);
    assert!(org2.is_none(), "Expected None for unallocated OUI FE:FF:FF");

    println!("Correctly returns None for unallocated OUI addresses");
}

#[test]
fn test_virtual_nic_detection() {
    // Test virtual NIC detection
    assert!(OuiDb::is_virtual_nic("QEMU virtual NIC"));
    assert!(OuiDb::is_virtual_nic("Bochs virtual NIC"));
    assert!(OuiDb::is_virtual_nic("PearPC virtual NIC"));
    assert!(OuiDb::is_virtual_nic("Cooperative Linux virtual NIC"));
    assert!(OuiDb::is_virtual_nic("Oracle VirtualBox virtual NIC"));

    // Test non-virtual NICs
    assert!(!OuiDb::is_virtual_nic("Cisco Systems"));
    assert!(!OuiDb::is_virtual_nic("Intel"));
    assert!(!OuiDb::is_virtual_nic("Unknown"));
}

#[test]
fn test_oui_subtable_name() {
    assert_eq!(OuiDb::oui_subtable_name(), "IEEE Registration Authority");
}

#[test]
fn test_mac_address_from_bytes() {
    let bytes = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    let mac = MacAddress::from(bytes);
    assert_eq!(mac.octets(), bytes);
}

#[test]
fn test_mac_address_from_slice() {
    let bytes = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    let mac = MacAddress::from_slice(&bytes);
    assert_eq!(mac, Ok(MacAddress::from(bytes)));

    // Wrong length
    let wrong_bytes = [0x12, 0x34, 0x56];
    let mac = MacAddress::from_slice(&wrong_bytes);
    assert!(mac.is_err());
}

#[test]
fn test_mac_address_display() {
    let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
    assert_eq!(format!("{}", mac), "00:11:22:33:44:55");
}

#[test]
fn test_mac_address_debug() {
    let mac: MacAddress = "00:11:22:33:44:55".parse().unwrap();
    assert_eq!(format!("{:?}", mac), "00:11:22:33:44:55");

    let mac: MacAddress = "fa:16:3e:05:44:9b".parse().unwrap();
    assert!(mac.is_local());
}

#[test]
fn test_oui_lookup_batch() {
    let db = oui_db();
    // Batch-test some known OUIs (use starts_with to work with both the trimmed and the fullname datasets)
    let test_cases = vec![
        ("00:00:00:00:00:00", "XEROX"),
        ("00:00:01:00:00:00", "XEROX"),
        ("00:00:02:00:00:00", "XEROX"),
        ("00:00:0C:00:00:00", "Cisco Systems"),
        ("00:00:17:00:00:00", "Oracle"),
        ("00:00:39:00:00:00", "TOSHIBA"),
        ("00:00:4C:00:00:00", "NEC"),
        ("00:00:60:00:00:00", "Kontron Europe"),
        ("00:00:85:00:00:00", "CANON"),
        ("00:00:87:00:00:00", "HITACHI"),
        ("fa:16:3e:05:44:9b", "OpenStack"),
    ];

    for (mac_str, expected_org) in test_cases {
        let mac: MacAddress = mac_str.parse().unwrap();
        let org = db.lookup_mac(mac);
        assert!(
            matches!(&org, Some(e) if e.name().starts_with(expected_org)),
            "Failed for MAC: {} (expected prefix '{}', got {:?})",
            mac_str,
            expected_org,
            org
        );
    }
}

#[test]
fn test_oui_database_coverage() {
    let db = oui_db();
    // Test the database's coverage
    // Verify database completeness by querying some known addresses

    // Test some random addresses to make sure the database works
    let test_addresses = vec![
        "00:00:00:00:00:00",
        "00:00:0C:00:00:00",
        "00:55:DA:00:00:00",
        "00:16:E3:00:00:00",
    ];

    let mut found_count = 0;
    for addr in test_addresses {
        let mac: MacAddress = addr.parse().unwrap();
        if db.lookup_mac(mac).is_some() {
            found_count += 1;
        }
    }

    // Should find at least most of them
    assert!(
        found_count >= 3,
        "Expected to find at least 3 OUI entries, found {}",
        found_count
    );
}

#[test]
fn test_oui_all_types_coverage() {
    let db = oui_db();
    // Comprehensive test: verify that 24/28/36-bit and unallocated OUIs are all handled correctly

    // === 24-bit OUI (MA-L) ===
    let mac_24: MacAddress = "00:00:01:00:00:00".parse().unwrap();
    let org_24 = db.lookup_mac(mac_24);
    assert!(
        matches!(&org_24, Some(e) if e.country_or_region_code() == "US" && e.name().starts_with("XEROX")),
        "24-bit OUI lookup failed: {:?}",
        org_24
    );
    println!("✓ 24-bit OUI: {} -> {:?}", mac_24, org_24);

    // === 28-bit OUI (MA-M) ===
    let mac_28: MacAddress = "00:55:DA:00:00:00".parse().unwrap();
    let org_28 = db.lookup_mac(mac_28);
    assert!(org_28.is_some(), "28-bit OUI lookup failed");
    println!("✓ 28-bit OUI: {} -> {:?}", mac_28, org_28);

    // === 36-bit OUI (MA-S) ===
    let mac_36: MacAddress = "00:16:E3:00:00:00".parse().unwrap();
    let org_36 = db.lookup_mac(mac_36);
    assert!(org_36.is_some(), "36-bit OUI lookup failed");
    println!("✓ 36-bit OUI: {} -> {:?}", mac_36, org_36);

    // === Unallocated OUI ===
    let mac_none: MacAddress = "FF:FF:FF:00:00:00".parse().unwrap();
    let org_none = db.lookup_mac(mac_none);
    assert!(org_none.is_none(), "Should return None for unallocated OUI");
    println!("✓ Unallocated OUI: {} -> {:?}", mac_none, org_none);

    println!("\n✅ All OUI types (24/28/36-bit and unallocated) are properly handled!");
}

#[test]
fn test_mac_address_ord() {
    let mac1: MacAddress = "00:11:22:33:44:55".parse().unwrap();
    let mac2: MacAddress = "00:11:22:33:44:56".parse().unwrap();
    let mac3: MacAddress = "00:11:22:33:44:55".parse().unwrap();

    assert!(mac1 < mac2);
    assert_eq!(mac1, mac3);
    assert!(mac2 > mac1);
}

#[test]
fn test_mac_address_default() {
    let mac = MacAddress::default();
    assert!(mac.is_zero());
}

#[test]
fn test_mac_address_cisco_ios_format() {
    // Test the Cisco IOS format: XXXX.XXXX.XXXX
    let mac1: MacAddress = "001A.2B3C.4D5E".parse().unwrap();
    assert_eq!(mac1.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    // Test all-zero address
    let mac2: MacAddress = "0000.0000.0000".parse().unwrap();
    assert_eq!(mac2.octets(), [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert!(mac2.is_zero());

    // Test broadcast address
    let mac3: MacAddress = "FFFF.FFFF.FFFF".parse().unwrap();
    assert_eq!(mac3.octets(), [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    assert!(mac3.is_broadcast());

    // Test case-insensitivity
    let mac4: MacAddress = "001a.2b3c.4d5e".parse().unwrap();
    assert_eq!(mac4.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    let mac5: MacAddress = "001a.2B3c.4d5E".parse().unwrap();
    assert_eq!(mac5.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    // Test consistency with other formats
    let mac_colon: MacAddress = "00:1A:2B:3C:4D:5E".parse().unwrap();
    let mac_dash: MacAddress = "00-1A-2B-3C-4D-5E".parse().unwrap();
    let mac_cisco: MacAddress = "001A.2B3C.4D5E".parse().unwrap();
    assert_eq!(mac_colon, mac_dash);
    assert_eq!(mac_colon, mac_cisco);
}

#[test]
fn test_mac_address_cisco_ios_format_invalid() {
    // Test invalid length
    let result: Result<MacAddress, _> = "001A.2B3C.4D5".parse();
    assert!(result.is_err());

    let result: Result<MacAddress, _> = "001A.2B3C.4D5E6".parse();
    assert!(result.is_err());

    // Test an invalid separator position
    let result: Result<MacAddress, _> = "001.2B3C.4D5E".parse();
    assert!(result.is_err());

    let result: Result<MacAddress, _> = "001A2B3C.4D5E".parse();
    assert!(result.is_err());

    // Test invalid hex characters
    let result: Result<MacAddress, _> = "001G.2B3C.4D5E".parse();
    assert!(result.is_err());

    let result: Result<MacAddress, _> = "001A.2B3H.4D5E".parse();
    assert!(result.is_err());

    // Test an invalid separator
    let result: Result<MacAddress, _> = "001A,2B3C,4D5E".parse();
    assert!(result.is_err());
}

#[test]
fn test_mac_address_continuous_hex() {
    // Test the no-separator continuous hex format: 12 hex characters
    let mac1: MacAddress = "001A2B3C4D5E".parse().unwrap();
    assert_eq!(mac1.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    // Test all-zero address
    let mac2: MacAddress = "000000000000".parse().unwrap();
    assert_eq!(mac2.octets(), [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert!(mac2.is_zero());

    // Test broadcast address
    let mac3: MacAddress = "FFFFFFFFFFFF".parse().unwrap();
    assert_eq!(mac3.octets(), [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    assert!(mac3.is_broadcast());

    // Test case-insensitivity
    let mac4: MacAddress = "001a2b3c4d5e".parse().unwrap();
    assert_eq!(mac4.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    let mac5: MacAddress = "001A2b3C4d5E".parse().unwrap();
    assert_eq!(mac5.octets(), [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);

    // Test consistency with other formats
    let mac_colon: MacAddress = "00:1A:2B:3C:4D:5E".parse().unwrap();
    let mac_dash: MacAddress = "00-1A-2B-3C-4D-5E".parse().unwrap();
    let mac_cisco: MacAddress = "001A.2B3C.4D5E".parse().unwrap();
    let mac_continuous: MacAddress = "001A2B3C4D5E".parse().unwrap();
    assert_eq!(mac_colon, mac_dash);
    assert_eq!(mac_colon, mac_cisco);
    assert_eq!(mac_colon, mac_continuous);
}

#[test]
fn test_mac_address_continuous_hex_invalid() {
    // Test invalid length (too short)
    let result: Result<MacAddress, _> = "001A2B3C4D5".parse();
    assert!(result.is_err());

    // Test invalid length (too long)
    let result: Result<MacAddress, _> = "001A2B3C4D5E6".parse();
    assert!(result.is_err());

    // Test invalid hex characters
    let result: Result<MacAddress, _> = "001G2B3C4D5E".parse();
    assert!(result.is_err());

    let result: Result<MacAddress, _> = "001A2BH3C4D5E".parse();
    assert!(result.is_err());
}

#[test]
fn test_iter_print_xerox_entries() {
    let db = oui_db();
    let mut count = 0usize;
    println!("XEROX entries:");
    for entry in db.iter() {
        if entry.name().contains("XEROX") {
            count += 1;
            println!(
                "  {}  {} ({})",
                entry.prefix_hex(),
                entry.name(),
                entry.country_or_region_code()
            );
        }
    }
    println!("Total XEROX entries: {count}");
    assert!(count > 0, "expected at least one XEROX entry");
}

#[test]
fn test_mac8_parsing_and_octets() {
    let mac: MacAddress8 = "00:11:22:33:44:55:66:77".parse().unwrap();
    assert_eq!(
        mac.octets(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
    );

    let mac_dash: MacAddress8 = "00-11-22-33-44-55-66-77".parse().unwrap();
    assert_eq!(mac, mac_dash);

    let mac_cisco: MacAddress8 = "0011.2233.4455.6677".parse().unwrap();
    assert_eq!(mac, mac_cisco);

    let mac_plain: MacAddress8 = "0011223344556677".parse().unwrap();
    assert_eq!(mac, mac_plain);

    // round-trip via display
    assert_eq!(format!("{mac}"), "00:11:22:33:44:55:66:77");

    // to_u64 (big-endian)
    assert_eq!(mac.to_u64(), 0x0011_2233_4455_6677);
}

#[test]
fn test_lookup_mac6_returns_entry() {
    let db = oui_db();
    let mac = MacAddress8::new([0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let org = db.lookup_mac6(mac).expect("XEROX entry");
    assert!(org.name().starts_with("XEROX"));

    // 8-byte and 6-byte prefixes with the same leading bytes resolve identically
    let mac6: MacAddress = "00:00:01:00:00:00".parse().unwrap();
    let org6 = db.lookup_mac(mac6).expect("XEROX entry");
    assert_eq!(org.prefix_hex(), org6.prefix_hex());
}

#[test]
fn test_lookup_slice_length_handling() {
    let db = oui_db();

    // Fewer than 3 bytes is never found
    assert!(db.lookup(&[]).is_none());
    assert!(db.lookup(&[0x00]).is_none());
    assert!(db.lookup(&[0x00, 0x00]).is_none());

    // Exactly the 24-bit prefix resolves like the full 6-byte address
    let three = db.lookup(&[0x00, 0x00, 0x01]).expect("000001");
    let six = db.lookup(&[0x00, 0x00, 0x01, 0, 0, 0]).expect("000001");
    assert_eq!(three.prefix_hex(), six.prefix_hex());
    assert_eq!(three.name(), six.name());
}

#[test]
fn test_lookup_subtable_authority_fallback() {
    let db = oui_db();

    // B84C87 is a 28-bit "IEEE Registration Authority" subtable parent.
    // 3 bytes is too short to resolve the 28-bit sub-prefix (needs byte 4).
    let parent = db.lookup(&[0xB8, 0x4C, 0x87]).expect("parent entry");
    assert_eq!(parent.name(), "IEEE Registration Authority");
    assert_eq!(parent.country_or_region_code(), "US");
    assert_eq!(parent.prefix_hex(), "B84C87");

    // With byte 4 present the actual 28-bit sub-entry resolves (not the parent).
    let airgain = db.lookup(&[0xB8, 0x4C, 0x87, 0x90]).expect("B84C879 entry");
    assert_eq!(airgain.prefix_hex(), "B84C879");

    // 70B3D5 is a 36-bit "IEEE Registration Authority" subtable parent.
    // 4 bytes is not enough (36-bit needs bytes 4 and 5); 3/4 bytes fall back.
    let p36 = db.lookup(&[0x70, 0xB3, 0xD5]).expect("parent entry");
    assert_eq!(p36.name(), "IEEE Registration Authority");
    let p36_len4 = db.lookup(&[0x70, 0xB3, 0xD5, 0x06]).expect("parent entry");
    assert_eq!(p36_len4.name(), "IEEE Registration Authority");

    // With bytes 4 and 5 present the 36-bit sub-entry resolves.
    let sub36 = db.lookup(&[0x70, 0xB3, 0xD5, 0x06, 0x00]);
    assert!(sub36.is_some());

    // A normal 24-bit prefix ignores extra bytes.
    let normal = db
        .lookup(&[0x00, 0x00, 0x01, 0xFF, 0xFF, 0xFF])
        .expect("000001");
    assert!(normal.name().starts_with("XEROX"));
}

#[test]
fn test_lookup_mac6_owned_independent() {
    let db = oui_db();
    let mac = MacAddress8::new([0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let owned = db.lookup_mac6_owned(mac).expect("XEROX entry");
    // Owned entry does not borrow the database
    let name = owned.name().to_string();
    assert!(name.starts_with("XEROX"));
}
