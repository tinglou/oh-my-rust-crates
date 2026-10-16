use chrono::Local;
use std::fs;
use std::io;
use std::path::Path;

const URLS: &[(&str, &str)] = &[
    (
        "https://standards-oui.ieee.org/oui/oui.csv",
        "oui.csv",
    ),
    (
        "https://standards-oui.ieee.org/iab/iab.csv",
        "iab.csv",
    ),
    (
        "https://standards-oui.ieee.org/oui36/oui36.csv",
        "oui36.csv",
    ),
    (
        "https://standards-oui.ieee.org/oui28/mam.csv",
        "mam.csv",
    ),
];

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let db_dir = Path::new("db");
    fs::create_dir_all(db_dir)?;

    for (url, filename) in URLS {
        let path = db_dir.join(filename);
        println!("Downloading {} -> {}", url, path.display());

        let resp = ureq::get(url)
            .set("User-Agent", "ouidb-builder/0.1")
            .call()?;
        let mut file = fs::File::create(&path)?;
        io::copy(&mut resp.into_reader(), &mut file)?;

        println!("  OK: {} bytes", path.metadata()?.len());
    }

    let timestamp = Local::now().format("%Y-%m-%d").to_string();
    fs::write(db_dir.join("timestamp.txt"), &timestamp)?;
    println!("Timestamp: {}", timestamp);

    Ok(())
}