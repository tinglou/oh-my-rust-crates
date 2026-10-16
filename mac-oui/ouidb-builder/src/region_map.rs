/// Region/city name → ISO 3166-1 alpha-2 country code.
/// Covers only the specific cases found in IEEE OUI data where addresses
/// lack a 2-letter country code but contain a recognizable city/region.
pub fn lookup_region(address: &str) -> Option<&'static str> {
    let upper = address.to_uppercase();

    // Chinese provinces/cities → CN
    if upper.contains("GUANGDONG")
        || upper.contains("BEIJING")
        || upper.contains("FUJIAN")
        || upper.contains("SHANGHAI")
        || upper.contains("DONGGUAN")
        || upper.contains("ZHEJIANG")
        || upper.contains("SHENZHEN")
        || upper.contains("TIANJIN")
        || upper.contains("JIANGSU")
        || upper.contains("XIAMEN")
    {
        return Some("CN");
    }

    // US states → US
    if upper.contains("CALIFORNIA") || upper.contains("NATICK") {
        return Some("US");
    }

    // Japanese cities → JP
    if upper.contains("TOKYO") {
        return Some("JP");
    }

    // German regions → DE
    if upper.contains("NRW") || upper.contains("HESSEN") {
        return Some("DE");
    }

    // Italian regions → IT
    if upper.contains("GENOVA") {
        return Some("IT");
    }

    // UK regions → GB
    if upper.contains("ESSEX") {
        return Some("GB");
    }

    // French regions → FR
    if upper.contains("IDF") {
        return Some("FR");
    }

    // Swedish regions → SE
    if upper.contains("GOTALAND") || upper.contains("GÖTALAND") {
        return Some("SE");
    }

    // Czech regions → CZ
    if upper.contains("VYSOCINA") {
        return Some("CZ");
    }

    None
}

/// Infer country from company name when address is empty or has no location info.
/// Covers only the specific cases found in IEEE OUI data.
pub fn lookup_company(name: &str) -> Option<&'static str> {
    let upper = name.to_uppercase();

    // Known companies
    if upper.contains("WIPRO") {
        return Some("IN");
    }
    if upper.contains("WANG LABORATORIES") {
        return Some("US");
    }
    if upper.contains("CONRAD") && !upper.contains("THOMAS CONRAD") {
        return Some("DE");
    }
    if upper.contains("BOSTON DYNAMICS") {
        return Some("US");
    }
    if upper.contains("TELENET") && !upper.contains("CHINA") {
        return Some("BE");
    }
    if upper.contains("RADA ELECTRONICS") {
        return Some("IL");
    }
    if upper.contains("LCR EMBEDDED") {
        return Some("US");
    }
    if upper.contains("CHARLES INDUSTRIES") {
        return Some("US");
    }
    if upper.contains("FRED KOSCHARA") {
        return Some("US");
    }
    if upper.contains("SANRITZ") {
        return Some("JP");
    }

    // Chinese cities in company names → CN
    if upper.contains("TIANJIN")
        || upper.contains("XIAMEN")
        || upper.contains("JIANGSU")
    {
        return Some("CN");
    }

    // "PTE. LTD." suffix → Singapore
    if upper.contains("PTE. LTD") || upper.contains("PTE LTD") {
        return Some("SG");
    }

    None
}