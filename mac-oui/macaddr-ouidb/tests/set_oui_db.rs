//! Tests the multiple-install semantics of `set_oui_db`
//!
//! Kept in a separate integration test file: each `tests/*.rs` compiles to its own
//! binary, isolating global state from other test processes.

use macaddr_ouidb::{OUI_DB_BUNDLED, OuiDb, OuiDbOwned, oui_db, reset_oui_db, set_oui_db};

#[test]
fn set_oui_db_can_replace_multiple_times() {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
    let custom = format!("{MANIFEST_DIR}/custom-mac-oui.txt");
    let ieee = format!("{MANIFEST_DIR}/ieee-mac-oui.txt");

    // By default it should be Bundled and match OUI_DB_BUNDLED
    assert!(
        matches!(&*oui_db(), OuiDb::Bundled(_)),
        "default should be Bundled before set_oui_db"
    );
    assert_eq!(oui_db().updated(), OUI_DB_BUNDLED.updated());

    // First install: Owned
    let owned = OuiDbOwned::from_files(&custom, &ieee).expect("load owned db");
    let expected_count = owned.iter().count();
    set_oui_db(owned.into());

    // The global should route to Owned
    assert!(
        matches!(&*oui_db(), OuiDb::Owned(_)),
        "should route to Owned"
    );

    // Lookup/iteration go through Owned, matching expectations (same source data as the bundled database, same entry count)
    assert_eq!(oui_db().iter().count(), expected_count);
    let mac = macaddr_ouidb::MacAddress::new([0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    assert!(
        oui_db()
            .lookup_mac(mac)
            .expect("XEROX entry")
            .name()
            .starts_with("XEROX")
    );

    // Hold a global handle, then replace the global database
    let held = oui_db();

    // Second install: allowed, the latest one wins (here switching back to Bundled); OUI_DB_BUNDLED is Copy
    set_oui_db(OuiDb::Bundled(OUI_DB_BUNDLED));
    assert!(
        matches!(&*oui_db(), OuiDb::Bundled(_)),
        "second set_oui_db should take effect"
    );

    // arc-swap semantics: the replaced Owned instance is kept alive by `held`, so the handle still queries correctly
    assert!(
        matches!(&*held, OuiDb::Owned(_)),
        "held OuiDbRef should keep the replaced Owned db alive"
    );
    assert_eq!(held.iter().count(), expected_count);
    assert!(
        held.lookup_mac(mac)
            .expect("XEROX entry")
            .name()
            .starts_with("XEROX")
    );
    drop(held);

    // Third install: switch back to Owned
    let owned2 = OuiDbOwned::from_files(&custom, &ieee).expect("reload owned db");
    set_oui_db(owned2.into());
    assert!(matches!(&*oui_db(), OuiDb::Owned(_)));
    assert_eq!(oui_db().iter().count(), expected_count);

    // reset back to the bundled database
    reset_oui_db();
    assert!(
        matches!(&*oui_db(), OuiDb::Bundled(_)),
        "reset_oui_db should restore the bundled database"
    );
    // reset is still available after resetting (idempotent)
    reset_oui_db();
    assert!(matches!(&*oui_db(), OuiDb::Bundled(_)));

    // Installation is still possible after resetting
    let owned3 = OuiDbOwned::from_files(&custom, &ieee).expect("reload owned db");
    set_oui_db(owned3.into());
    assert!(matches!(&*oui_db(), OuiDb::Owned(_)));
}
