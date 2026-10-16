use std::collections::HashSet;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

const CSV_FILES: &[&str] = &["db/oui.csv", "db/iab.csv", "db/oui36.csv"];

const SOURCE_URLS: &[(&str, &str)] = &[
    ("MA-L(24 bits)", "https://standards-oui.ieee.org/oui/oui.csv"),
    ("MA-M(28 bits)", "https://standards-oui.ieee.org/iab/iab.csv"),
    ("MA-S(36 bits)", "https://standards-oui.ieee.org/oui36/oui36.csv"),
];

const OUTPUT_DEFAULT: &str = "../macaddr-ouidb/ieee-mac-oui.txt";
const OUTPUT_FULLNAME: &str = "../macaddr-ouidb/ieee-mac-oui-fullname.txt";

const TIMESTAMP_FILE: &str = "db/timestamp.txt";

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

/// Corporate suffixes ordered longest-first for greedy matching.
/// Each entry is the normalized (lowercase, no punctuation) form.
const CORP_SUFFIXES: &[&str] = &[
    "gmbh & co kg",     // Germany: GmbH & Co. KG
    "gmbh & co",        // Germany: GmbH & Co. (truncated)
    "gmbh co kg",       // Germany: GmbH Co. KG (no &)
    "sdn bhd",          // Malaysia: Sendirian Berhad
    "co ltd",           // China: Co., Ltd. / Co.,Ltd
    "corp ltd",         // China: Corp., Ltd.
    "pte ltd",          // Singapore: Pte. Ltd.
    "pty ltd",          // Australia: Pty. Ltd.
    "pvt ltd",          // India: Pvt. Ltd. (Private Limited)
    "s p a",            // Italy: S.p.A.
    "s r l",            // Italy: S.r.l.
    "s r o",            // Czech: s.r.o.
    "s a s",            // France: SAS (simplified stock corp)
    "s a u",            // Spain: S.A.U.
    "b v",              // Netherlands: B.V. (dotted variant)
    "n v",              // Netherlands/Belgium: N.V. (dotted variant)
    "k k",              // Japan: Kabushiki Kaisha (K.K.)
    "g k",              // Japan: Gōdō Kaisha (G.K.)
    "corporation",      // USA: Corporation
    "incorporated",     // USA: Incorporated
    "corporate",        // USA: Corporate
    "limited liability company", // full form: Limited Liability Company
    "ltd co",           // e.g. "Ltd. Co."
    "limited",          // UK: Limited
    "spa",              // Italy: S.p.A (alternate form)
    "llc",              // USA: Limited Liability Company
    "llp",              // USA: Limited Liability Partnership
    "ltd",              // UK: Ltd / Ltd.
    "plc",              // UK: Public Limited Company
    "gmbh",             // Germany: GmbH
    "corp",             // USA: Corp / Corp.
    "inc",              // USA: Inc / Inc.
    "l p",              // USA: Limited Partnership
    "ag",               // Germany/Switzerland: AG
    "bv",               // Netherlands: BV
    "nv",               // Netherlands/Belgium: NV
    "kg",               // Germany: KG (limited partnership)
    "kft",              // Hungary: Korlátolt Felelősségű Társaság (LLC)
    "zrt",              // Hungary: Zártkörűen Működő Részvénytársaság (private)
    "nyrt",             // Hungary: Nyilvánosan Működő Részvénytársaság (public)
    "bt",               // Hungary: Betéti Társaság (limited partnership)
    "kkt",              // Hungary: Közkereseti Társaság (general partnership)
    "ab",               // Sweden: Aktiebolag
    "oy",               // Finland: Osakeyhtiö
    "oyj",              // Finland: Julkinen osakeyhtiö
    "aps",              // Denmark: ApS
    "as",               // Norway: AS / Denmark: A/S
    "a s",              // Denmark/Norway: A/S (alternate form)
    "s a",              // France/Spain/Latin: S.A. / SA
    "ulc",              // Canada: Unlimited Liability Company
    "pc",               // USA: Professional Corporation
];

/// Split a name into normalized lowercase word tokens (strips all punctuation).
fn normalize_words(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Trim known corporate suffixes from the end of an organization name.
fn trim_corp_suffix(name: &str) -> String {
    let words = normalize_words(name);
    if words.len() < 2 {
        return name.to_string();
    }

    // Try matching from longest suffix first
    for suffix in CORP_SUFFIXES {
        let suffix_words: Vec<&str> = suffix.split_whitespace().collect();
        let n = suffix_words.len();
        if words.len() <= n {
            continue;
        }

        let tail = &words[words.len() - n..];
        if tail != suffix_words.as_slice() {
            continue;
        }

        // Build (start_pos, end_pos) for each normalized word in the original
        let mut word_spans: Vec<(usize, usize)> = Vec::with_capacity(words.len());
        let mut in_word = false;
        let mut start = 0usize;

        for (i, c) in name.char_indices() {
            if c.is_ascii_alphanumeric() && !in_word {
                start = i;
                in_word = true;
            } else if !c.is_ascii_alphanumeric() && in_word {
                in_word = false;
                word_spans.push((start, i));
            }
        }
        if in_word {
            word_spans.push((start, name.len()));
        }

        let keep_count = words.len() - n;
        // Validate the word spans match our normalized words
        if word_spans.len() < words.len()
            || word_spans
                .iter()
                .zip(words.iter())
                .any(|((s, e), w)| name[*s..*e].to_lowercase() != *w)
        {
            continue;
        }

        // Cut after the last keep word (end position of that span),
        // then keep trailing non-alnum chars that belong to the keep word
        // (e.g., closing paren) but stop before the suffix separator.
        let last_keep_end = word_spans[keep_count - 1].1;
        let suffix_start = word_spans[keep_count].0;

        // Walk chars backwards from suffix_start to find the cut point
        // (skip trailing whitespace/commas/dots between keep words and suffix).
        let mut cut_pos = suffix_start;
        for (j, c) in name[last_keep_end..suffix_start].char_indices().rev() {
            if c.is_whitespace() || c == ',' || c == '.' {
                cut_pos = last_keep_end + j;
            } else {
                break;
            }
        }

        // If nothing remains, return original
        if cut_pos == 0 {
            return name.to_string();
        }

        return name[..cut_pos]
            .trim_end_matches(|c: char| c == ',' || c == '.' || c.is_whitespace())
            .to_string();
    }

    name.to_string()
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
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut unique_names: HashSet<String> = HashSet::new();
    let mut unique_trimmed: HashSet<String> = HashSet::new();
    let mut saved_bytes = 0usize;

    // Collect all lines in memory so we can write the header after analysis
    let mut lines: Vec<String> = Vec::new();
    let mut unfixable: Vec<(String, String)> = Vec::new(); // (mac_prefix, org_name)

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
            let org_name = record[2].to_string();
            let address = &record[3];

            let country = extract_country_code(address, &org_name);
            let org_name_trimmed = if keep_suffix {
                org_name.clone()
            } else {
                trim_corp_suffix(&org_name)
            };

            if org_name_trimmed != org_name {
                trimmed_count += 1;
                unique_trimmed.insert(org_name.clone());
                if seen_names.insert(org_name.clone()) {
                    saved_bytes += org_name.len() - org_name_trimmed.len();
                    println!("  {} -> {}", org_name, org_name_trimmed);
                }
            }
            unique_names.insert(org_name_trimmed.clone());

            if country == "--" && org_name != "Private" {
                unfixable.push((mac_prefix.clone(), org_name.clone()));
            }

            lines.push(format!("{} {} {}", mac_prefix, country, org_name_trimmed));
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
    println!("Done: {} entries written to {}", total, output);
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