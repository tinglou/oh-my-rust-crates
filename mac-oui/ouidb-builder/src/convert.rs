use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

const CSV_FILES: &[&str] = &["db/oui.csv", "db/iab.csv", "db/oui36.csv", "db/mam.csv"];

const SOURCE_URLS: &[(&str, &str)] = &[
    ("MA-L(24 bits)", "https://standards-oui.ieee.org/oui/oui.csv"),
    ("MA-M(28 bits)", "https://standards-oui.ieee.org/oui28/mam.csv"),
    ("MA-S(36 bits)", "https://standards-oui.ieee.org/oui36/oui36.csv"),
    ("IAB(36 bits)", "https://standards-oui.ieee.org/iab/iab.csv"),
];

const OUTPUT_DEFAULT: &str = "../macaddr-ouidb/ieee-mac-oui.txt";
const OUTPUT_FULLNAME: &str = "../macaddr-ouidb/ieee-mac-oui-fullname.txt";

const TIMESTAMP_FILE: &str = "db/timestamp.txt";
const TRIM_LOG: &str = "trim-suffix.log";
const FIX_TYPO_LOG: &str = "fix-typo.log";
const DUPLICATE_LOG: &str = "duplicate-prefix.log";

/// ISO 3166-1 alpha-2 country codes. Sorted for binary search.
/// Used to validate extracted 2-letter tokens are actual country codes,
/// filtering out state/province abbreviations (NY, CA-as-state, etc.).
const ISO_3166_ALPHA2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
    "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ",
    "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK",
    "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
    "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR",
    "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS",
    "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN",
    "IO", "IQ", "IR", "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
    "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV",
    "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ",
    "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI",
    "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
    "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC",
    "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV",
    "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR",
    "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

fn is_known_country(code: &str) -> bool {
    ISO_3166_ALPHA2.binary_search(&code).is_ok()
}

/// Replace Unicode space separators (Zs category, e.g. NBSP, en space, ideographic
/// space) with a regular ASCII space. Only U+0020 is kept as-is among spaces.
fn is_unicode_space(c: char) -> bool {
    matches!(c,
        '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// Strip invisible Unicode characters from IEEE-supplied organization names.
///
/// IEEE CSV data is vendor-submitted and contains various invisible artifacts:
/// - Unicode space separators (NBSP, en space, etc.): replaced with ASCII space
/// - U+00AD (soft hyphen): removed
/// - U+200B..U+200F (zero-width/bidi marks): removed
/// - U+2060..U+2064, U+2066..U+206F (word joiner, format controls): removed
/// - U+FEFF (BOM/ZWNBSP): removed
/// - U+061C (ALM): removed
/// - U+202A..U+202E (bidi embedding/override): removed
///
/// Strategy: map chars to Option<char>, then flatten. Normal chars pass through,
/// space separators become ASCII space, invisible chars become None (removed).
fn sanitize_org_name(name: &str) -> String {
    name.chars()
        .filter_map(|c| {
            if is_unicode_space(c) {
                Some(' ')
            } else if c == '\u{00AD}'
                || ('\u{200B}'..='\u{200F}').contains(&c)
                || ('\u{202A}'..='\u{202E}').contains(&c)
                || ('\u{2060}'..='\u{2064}').contains(&c)
                || ('\u{2066}'..='\u{206F}').contains(&c)
                || c == '\u{FEFF}'
                || c == '\u{061C}'
            {
                None
            } else {
                Some(c)
            }
        })
        .collect()
}

/// Map spelled-out country/region names to ISO 3166-1 alpha-2 codes.
fn lookup_country_name(word: &str) -> Option<&'static str> {
    // Match against the FULL address, since "SOUTH" alone can mean South Africa or South Korea
    // We look for the most specific pattern first.
    let upper = word.to_uppercase();
    match upper.as_str() {
        "ENGLAND" | "SCOTLAND" | "WALES" | "BRITAIN" => Some("GB"),
        "UNITED" => if word.contains("KINGDOM") { Some("GB") } else { None },
        "CANADA" => Some("CA"),
        "KOREA" => Some("KR"),
        "TAIWAN" | "R.O.C." => Some("TW"),
        "GERMANY" | "W-GERMANY" | "WESTGERMANY" => Some("DE"),
        "JAPAN" => Some("JP"),
        "CHINA" | "P.R.CHINA" | "P.R.CHINA." => Some("CN"),
        "ITALY" | "ITALIA" => Some("IT"),
        "FRANCE" | "FRANC" => Some("FR"),
        "SWEDEN" => Some("SE"),
        "SWITZERLAND" => Some("CH"),
        "AUSTRIA" => Some("AT"),
        "BELGIUM" => Some("BE"),
        "DENMARK" => Some("DK"),
        "FINLAND" => Some("FI"),
        "NORWAY" => Some("NO"),
        "POLAND" => Some("PL"),
        "HUNGARY" => Some("HU"),
        "SINGAPORE" => Some("SG"),
        "AUSTRALIA" => Some("AU"),
        "BRAZIL" => Some("BR"),
        "INDIA" => Some("IN"),
        "MEXICO" => Some("MX"),
        "IRELAND" => Some("IE"),
        "NETHERLANDS" | "HOLLAND" => Some("NL"),
        "THAILAND" => Some("TH"),
        "RUSSIA" => Some("RU"),
        "SLOVAKIA" => Some("SK"),
        "SERBIA" => Some("RS"),
        "SPAIN" => Some("ES"),
        "VIETNAM" => Some("VN"),
        "MALAYSIA" => Some("MY"),
        "TURKEY" => Some("TR"),
        "ISRAEL" => Some("IL"),
        "CZECH" => Some("CZ"),
        "PHILIPPINES" => Some("PH"),
        "INDONESIA" => Some("ID"),
        "PORTUGAL" => Some("PT"),
        "GREECE" => Some("GR"),
        "ROMANIA" => Some("RO"),
        "UKRAINE" => Some("UA"),
        _ => None,
    }
}

/// Extract 2-letter country code from the Organization Address field.
/// Scans whitespace-separated tokens from right to left, looking for the
/// last 2-letter ASCII uppercase token that is NOT part of a postal code
/// (to avoid matching Dutch "NL 1234 AB" style codes).
fn extract_country_code<'a>(address: &'a str, org_name: &str) -> &'a str {
    let tokens: Vec<&str> = address.split_whitespace().collect();

    let mut candidate = None; // (token, preceded_by_digits)

    for i in (0..tokens.len()).rev() {
        let t = tokens[i];

        // Handle hyphenated codes like "DK-6430", "B-2650", "DE-76437", "D-25337"
        if t.contains('-') {
            for part in t.rsplit('-') {
                if part.len() == 2
                    && part.bytes().all(|b| b.is_ascii_uppercase())
                    && is_known_country(part)
                {
                    return part;
                }
            }
            // Single-letter country prefixes: D- = Germany, F- = France, I- = Italy
            if let Some(prefix) = t.split('-').next() {
                match prefix {
                    "D" => return "DE",
                    "F" => return "FR",
                    "I" => return "IT",
                    "E" => return "ES",
                    "B" => return "BE",
                    "A" => return "AT",
                    _ => {}
                }
            }
        }

        if t.len() != 2 || !t.bytes().all(|b| b.is_ascii_uppercase()) {
            continue;
        }
        if !is_known_country(t) {
            // Handle legacy/transitional codes and US state abbreviations
            match t {
                "YU" | "CS" => return "RS", // Yugoslavia / Serbia-Montenegro → Serbia
                "SU" => return "RU",        // Soviet Union → Russia
                // US state codes that appear without country code
                "AL" | "AK" | "AZ" | "AR" | "CA" | "CO" | "CT" | "DE" | "FL" | "GA"
                | "HI" | "ID" | "IL" | "IN" | "IA" | "KS" | "KY" | "LA" | "ME" | "MD"
                | "MA" | "MI" | "MN" | "MS" | "MO" | "MT" | "NE" | "NV" | "NH" | "NJ"
                | "NM" | "NY" | "NC" | "ND" | "OH" | "OK" | "OR" | "PA" | "RI" | "SC"
                | "SD" | "TN" | "TX" | "UT" | "VT" | "VA" | "WA" | "WV" | "WI" | "WY"
                | "DC" | "AS" | "GU" | "MP" | "PR" | "VI" => return "US",
                _ => continue,
            }
        }
        let preceded_by_digits = i > 0 && tokens[i - 1].bytes().all(|b| b.is_ascii_digit());

        if !preceded_by_digits {
            return t; // Best match: valid country code not part of a postal code
        }
        if candidate.is_none() {
            candidate = Some(t); // Fallback: postal code suffix, but keep as last resort
        }
    }

    // Try spelled-out country names as fallback
    if candidate.is_none() {
        let addr_upper = address.to_uppercase();
        // Multi-word patterns first
        if addr_upper.contains("SOUTH AFRICA") {
            return "ZA";
        }
        if addr_upper.contains("SOUTH KOREA") || addr_upper.contains("REPUBLIC OF KOREA") {
            return "KR";
        }
        if addr_upper.contains("NEW ZEALAND") {
            return "NZ";
        }
        if addr_upper.contains("SAUDI ARABIA") {
            return "SA";
        }
        if addr_upper.contains("UNITED KINGDOM") {
            return "GB";
        }
        if addr_upper.contains("UNITED ARAB EMIRATES") || addr_upper.contains("U.A.E") {
            return "AE";
        }
        if addr_upper.contains("UNITED STATES") || addr_upper.contains("U.S.A") {
            return "US";
        }
        if addr_upper.contains("SRI LANKA") {
            return "LK";
        }
        if addr_upper.contains("COSTA RICA") {
            return "CR";
        }
        if addr_upper.contains("PUERTO RICO") {
            return "PR";
        }
        if addr_upper.contains("HONG KONG") {
            return "HK";
        }
        // Single-word fallback
        for t in tokens {
            if let Some(code) = lookup_country_name(t) {
                return code;
            }
        }
    }

    // Try region name → country inference as last resort
    if let Some(code) = crate::region_map::lookup_region(address) {
        return code;
    }

    // Try company name → country inference for empty/missing addresses
    if let Some(code) = crate::region_map::lookup_company(org_name) {
        return code;
    }

    candidate.unwrap_or("--")
}

pub fn run(keep_suffix: bool) -> Result<(), Box<dyn std::error::Error>> {
    let output = if keep_suffix { OUTPUT_FULLNAME } else { OUTPUT_DEFAULT };
    let out_dir = Path::new(output).parent().unwrap();
    fs::create_dir_all(out_dir)?;

    let mut total = 0usize;
    let mut trimmed_count = 0usize;
    let mut unique_names: BTreeSet<String> = BTreeSet::new();
    let mut unique_trimmed: BTreeSet<String> = BTreeSet::new();
    let mut saved_bytes = 0usize;

    // Collect all lines in memory so we can write the header after analysis
    let mut lines: Vec<String> = Vec::new();
    let mut unfixable: Vec<(String, String)> = Vec::new(); // (mac_prefix, org_name)

    // Unique organization-name changes, kept sorted (BTreeMap) for the log:
    // "original -> result". Written alphabetically after processing, to
    // trim-suffix.log (default) or fix-typo.log (--keep-suffix).
    let mut change_map: BTreeMap<String, String> = BTreeMap::new();

    // Duplicate prefix handling: IEEE reassigns prefixes over time, so the same
    // prefix can appear multiple times in the source CSVs. We keep the LAST
    // occurrence (most recent allocation) and record the dropped ones.
    // prefix -> index into `lines`
    let mut prefix_index: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    // prefix -> dropped lines (earlier occurrences, in source order)
    let mut duplicate_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut duplicate_occurrences = 0usize;

    for csv_path in CSV_FILES {
        let path = Path::new(csv_path);
        if !path.exists() {
            eprintln!("Warning: {} not found, skipping", csv_path);
            continue;
        }

        println!("Processing {}...", csv_path);
        let mut reader = csv::Reader::from_path(path)?;

        for result in reader.records() {
            let record = result?;

            let mac_prefix = record[1].to_string();
            let org_name = sanitize_org_name(record[2].trim());
            let address = &record[3];

            let country = extract_country_code(address, &org_name);
            let org_name_trimmed = if keep_suffix {
                crate::corp_suffix::fix_fullname(&org_name)
            } else {
                crate::corp_suffix::trim(&org_name)
            };

            if org_name_trimmed != org_name {
                if !keep_suffix {
                    trimmed_count += 1;
                    unique_trimmed.insert(org_name.clone());
                }
                if change_map
                    .insert(org_name.clone(), org_name_trimmed.clone())
                    .is_none()
                {
                    if !keep_suffix {
                        saved_bytes += org_name.len().saturating_sub(org_name_trimmed.len());
                    }
                    println!("  {} -> {}", org_name, org_name_trimmed);
                }
            }
            unique_names.insert(org_name_trimmed.clone());

            if country == "--" && org_name != "Private" {
                unfixable.push((mac_prefix.clone(), org_name.clone()));
            }

            let line = format!("{} {} {}", mac_prefix, country, org_name_trimmed);
            match prefix_index.get(&mac_prefix).copied() {
                Some(idx) => {
                    // Prefix already seen: keep the LAST occurrence (most recent
                    // allocation). Record the replaced (dropped) line if it differs.
                    duplicate_occurrences += 1;
                    let dropped = std::mem::replace(&mut lines[idx], line);
                    if dropped != lines[idx] {
                        duplicate_map.entry(mac_prefix).or_default().push(dropped);
                    }
                }
                None => {
                    prefix_index.insert(mac_prefix, lines.len());
                    lines.push(line);
                }
            }
            total += 1;
        }
    }

    // Write output file
    let out_file = fs::File::create(output)?;
    let mut writer = BufWriter::new(out_file);

    writeln!(writer, "# Organizationally Unique Identifiers (OUIs)")?;
    writeln!(writer, "# Original data comes from: ")?;
    let timestamp = fs::read_to_string(TIMESTAMP_FILE).unwrap_or_default();
    let date = timestamp.trim();
    for (name, url) in SOURCE_URLS {
        writeln!(writer, "# {}: {}", name, url)?;
    }
    writeln!(writer, "# Updated: {}", date)?;
    writeln!(
        writer,
        "# Duplicate prefixes are deduplicated: the LAST occurrence (most recent"
    )?;
    writeln!(
        writer,
        "# allocation) is kept. Dropped entries are logged to {}.",
        DUPLICATE_LOG
    )?;

    if !unfixable.is_empty() {
        writeln!(writer, "#")?;
        writeln!(writer, "# Entries with unresolved country code:")?;
        for (prefix, name) in &unfixable {
            writeln!(
                writer,
                "#   {} {} (empty address, no location clue)",
                prefix, name
            )?;
        }
    }

    for line in &lines {
        writeln!(writer, "{}", line)?;
    }

    writer.flush()?;

    // Write the change log alphabetically sorted by original name.
    // Default mode logs suffix trims; --keep-suffix logs typo/entity fixes.
    let log_path = if keep_suffix { FIX_TYPO_LOG } else { TRIM_LOG };
    let mut log = BufWriter::new(fs::File::create(log_path)?);
    for (original, result) in &change_map {
        writeln!(log, "{} -> {}", original, result)?;
    }
    log.flush()?;

    // Write the duplicate-prefix log (only if there were any duplicates).
    if !duplicate_map.is_empty() {
        let mut dup_log = BufWriter::new(fs::File::create(DUPLICATE_LOG)?);
        writeln!(
            dup_log,
            "# Duplicate OUI/prefix assignments in the source data (IEEE reassignments)."
        )?;
        writeln!(
            dup_log,
            "# Policy: keep the LAST occurrence (most recent allocation), drop earlier ones."
        )?;
        writeln!(dup_log, "# Format: <prefix> (bits)")?;
        writeln!(dup_log, "#           kept:    <kept entry>")?;
        writeln!(dup_log, "#           dropped: <dropped entry>")?;
        writeln!(dup_log, "#")?;
        for (prefix, dropped_lines) in &duplicate_map {
            let bits = match prefix.len() {
                6 => 24,
                7 => 28,
                9 => 36,
                _ => 0,
            };
            writeln!(dup_log, "{} ({} bits)", prefix, bits)?;
            if let Some(&idx) = prefix_index.get(prefix) {
                writeln!(dup_log, "  kept:    {}", lines[idx])?;
            }
            for dropped in dropped_lines {
                writeln!(dup_log, "  dropped: {}", dropped)?;
            }
        }
        dup_log.flush()?;
    }

    println!(
        "Done: {} entries written to {} ({} duplicate occurrences across {} prefixes)",
        lines.len(),
        output,
        duplicate_occurrences,
        duplicate_map.len()
    );
    if total > 0 && !keep_suffix {
        println!(
            "Trimmed: {}/{} ({:.1}%), unique: {}/{} ({:.1}%), saved: {} bytes",
            trimmed_count,
            total,
            trimmed_count as f64 / total as f64 * 100.0,
            unique_trimmed.len(),
            unique_names.len(),
            unique_trimmed.len() as f64 / unique_names.len() as f64 * 100.0,
            saved_bytes,
        );
    }
    Ok(())
}