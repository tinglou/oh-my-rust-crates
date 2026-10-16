/// Corporate suffixes ordered longest-first for greedy matching.
/// Each entry is the normalized (lowercase, no punctuation) form.
const CORP_SUFFIXES: &[&str] = &[
    "gmbh & co kg",     // Germany: GmbH & Co. KG
    "gmbh & co",        // Germany: GmbH & Co. (truncated)
    "gmbh co kg",       // Germany: GmbH Co. KG (no &)
    "gmbh and co kg",   // Germany: GmbH and Co. KG ("and" instead of "&")
    "ag co kg",         // Germany: AG & Co. KG (AG instead of GmbH)
    "co kg",            // Germany: & Co. KG (generic form)
    "co gmbh",          // Germany: Co. GmbH (+Co.GmbH)
    "co limited",       // UK: Co., Limited
    "co llc",           // USA: Co., LLC
    "sdn bhd",          // Malaysia: Sendirian Berhad
    "co ltd",           // China: Co., Ltd. / Co.,Ltd
    "co lt",            // Truncated Co.,Lt (missing final 'd')
    "co inc",           // Co.,Inc (combined Co. + Inc. without space)
    "a d ltd",          // Combined A.D. Ltd.
    "corp ltd",         // China: Corp., Ltd.
    "pte ltd",          // Singapore: Pte. Ltd.
    "pty ltd",          // Australia: Pty. Ltd.
    "pvt ltd",          // India: Pvt. Ltd. (Private Limited)
    "s p a",            // Italy: S.p.A.
    "sp z o o sp k",    // Poland: Sp. z o.o. Sp. k.
    "sp z o o",         // Poland: Sp. z o.o. (Spółka z o.o.)
    "sp k",             // Poland: Sp. k. (Spółka komandytowa)
    "s r l",            // Italy: S.r.l.
    "s r o",            // Czech: s.r.o.
    "s a s",            // France: SAS (simplified stock corp)
    "s a u",            // Spain: S.A.U.
    "s a r l",          // France: S.A.R.L.
    "s a de c v",       // Mexico: S.A. de C.V.
    "s l u",            // Spain: S.L.U. (Sociedad Limitada Unipersonal)
    "s l",              // Spain: S.L. (Sociedad Limitada)
    "s p r l",          // Belgium: S.p.R.L.
    "s c",              // Poland: s.c. (spółka cywilna)
    "s k a",            // Poland: S.K.A. (spółka komandytowo-akcyjna)
    "z s p o",          // Czech: z.s.p.o. (zapsaný spolek)
    "d o o",            // Croatia/Serbia/Bosnia: d.o.o. (LLC)
    "d d",              // Slovenia/Croatia: d.d. (delniška družba)
    "w l l",            // Bahrain/Qatar: W.L.L. (With Limited Liability)
    "l l c",            // USA: L.L.C. (dotted variant)
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
    "a s",              // Denmark/Norway: A/S (alternate form, must be before "as")
    "as",               // Norway: AS
    "s a",              // France/Spain/Latin: S.A. / SA
    "ulc",              // Canada: Unlimited Liability Company
    "pc",               // USA: Professional Corporation
    "se",               // EU: Societas Europaea (European Company)
    "co",               // standalone "Co." / "CO"
    "ltda",             // Brazil/Portugal: Ltda. / LTDA.
    "e v",              // Germany: e.V. (eingetragener Verein)
    "e k",              // Germany: e.K. (eingetragener Kaufmann)
    "e u",              // Austria: e.U. (eingetragener Unternehmer)
    "ges m b h",        // Austria: Ges.m.b.H.
    "sp j",             // Poland: sp.j. (spółka jawna)
    "s j",              // Poland: S.J. / s.j. (variant of sp.j.)
    "s a s di c",       // Italy: S.a.s. di C. (Società in accomandita semplice)
    "a d",              // Serbia: A.D. (Akcionarsko Društvo)
    "a b",              // Sweden: A.B. (Aktiebolag, dotted variant)
    "ehf",              // Iceland: ehf. (einkahlutafélag)
    "hb",               // Sweden: HB (Handelsbolag)
];

/// Hardcoded fixes for data quality issues: typos, truncations, and stray punctuation.
/// Non-ASCII org names that the ASCII-only tokenizer can't handle.
/// Fix truncated "Internation" -> "International" without corrupting the
/// already-correct "International" (which starts with the same prefix).
fn fix_truncated_internation(s: &str) -> String {
    const NEEDLE: &str = "Internation";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find(NEEDLE) {
        let after = &rest[pos + NEEDLE.len()..];
        out.push_str(&rest[..pos]);
        if after.starts_with("al") {
            // Already "International": keep as-is.
            out.push_str(NEEDLE);
        } else {
            out.push_str("International");
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

fn fix_name(name: &str) -> String {
    // Typo fixes: known misspellings in IEEE data
    let fixed = name
        .replace("&amp;", "&")          // HTML entity
        .replace("&quot;", "\"")        // HTML entity
        .replace("&#39;", "'")          // HTML entity (apostrophe)
        .replace("&apos;", "'")         // HTML entity (apostrophe)
        .replace("&b Co", "& Co")       // stray 'b' in "&b Co." (not "d&b ...")
        .replace('（', "(")              // fullwidth left paren
        .replace('）', ")")              // fullwidth right paren
        .replace("Develoment", "Development")
        .replace("Teachnologies", "Technologies")
        .replace("Commnunications", "Communications")
        .replace("Tecnology", "Technology")
        .replace(" Lmd.", " Ltd.")
        .replace(" Lmt.", " Ltd.")
        .replace(" ILNC.", " Inc.")
        .replace(" Incl.", " Inc.")
        .replace(" Co., Itd.", " Co., Ltd.")
        .replace(" Co.,Lte", " Co.,Ltd")
        .replace(" CO.,TD.", " Co.,Ltd.")
        .replace(" CO,.Ltd", " Co., Ltd.")
        .replace(" Co.Lltd", " Co.,Ltd")
        .replace(" Co.,Lid.", " Co.,Ltd.")
        .replace(" Crop.", " Corp.")
        // Hardcoded: "Internationa" missing 'l' + "C0." digit-zero for 'o'
        .replace(
            "Ficosa Internationa(Taicang) C0.,Ltd.",
            "Ficosa International (Taicang) Co., Ltd.",
        );

    // "Internation" is a common truncation of "International", but a plain
    // replace also matches the prefix of the already-correct word and turns
    // "International" into "Internationalal". Only fix it when not followed by "al".
    let fixed = fix_truncated_internation(&fixed);

    // Fix truncated "Co.,L" at end
    let fixed = if fixed.ends_with(" Co.,L") {
        fixed.replace(" Co.,L", " Co.,Ltd")
    } else {
        fixed
    };

    // Strip stray trailing comma (e.g. "PSI Software SE,")
    let fixed = fixed
        .trim_end_matches(',')
        .trim_end_matches('&')
        .trim_end()
        .to_string();

    // Strip stuck-on location suffixes: "Co.,Ltd.(CHINA)", ".Hefei", ".ShenZhen"
    let fixed = fixed
        .replace("Co.,Ltd.(CHINA)", "Co., Ltd.")
        .replace("Co.,Ltd.(NHN)", "Co., Ltd.")
        .replace("Co.,Ltd.Hefei", "Co., Ltd.")
        .replace("Co.,Ltd.ShenZhen", "Co., Ltd.")
        .replace("Co,Ltd.ShenZhen", "Co., Ltd.")
        .replace("Ltd.(China)", "Ltd.")
        .replace("Ltd.,UK", "Ltd.")
        .replace("CO.,LTD,HangZhou P.R.CHINA", "Co., Ltd.");

    // Strip trailing period on non-suffix single words
    

    fixed
        .replace(" system.", " System")
        .replace(" Network.", " Network")
}

/// Necessary conversion for the fullname (--keep-suffix) output: keep the
/// organization name as-is apart from HTML entities (e.g. "&amp;"), fullwidth
/// parens, known typos and collapsed whitespace. No suffix trimming.
pub fn fix_fullname(name: &str) -> String {
    normalize_spaces(&fix_name(name)).trim().to_string()
}

/// Hardcoded full-name mappings for entries with non-ASCII characters
/// that break the ASCII-only word tokenizer.
fn hardcoded_trim(name: &str) -> Option<&'static str> {
    match name {
        "Lechpol Electronics Spółka z o.o. Sp.k." => Some("Lechpol Electronics"),
        "VERIDAS Digital Authentication Solutions S.L" => Some("VERIDAS Digital Authentication Solutions"),
        // F.I.T. = Faithful Inventive Technologies: a proprietary brand, not a
        // legal suffix, so it must be kept (only "Co., Ltd." is trimmed).
        "Niigata Development Center, F.I.T. Co., Ltd." => Some("Niigata Development Center, F.I.T."),
        // Preserve context for names that become too generic when trimmed to one word
        "Code Corporation" => Some("Code Corp"),
        "LORD Corporation" => Some("LORD Corp"),
        // Remove redundant legal suffixes both inside and outside the parenthetical
        "S Digm Inc. (Safe Paradigm Inc.)" => Some("S Digm (Safe Paradigm)"),
        // Turkish non-ASCII Ş patterns
        "Nanotek Elektronik Sistemler Ltd. Sti." => Some("Nanotek Elektronik Sistemler"),
        "EDS Elektronik Destek San.Tic.Ltd.Sti" => Some("EDS Elektronik Destek"),
        "KENTKART EGE ELEKTRONIK SAN. VE TIC. LTD. STI." => Some("KENTKART EGE ELEKTRONIK"),
        "DTS ELEKTRONIK SAN. TIC. LTD. STI." => Some("DTS ELEKTRONIK"),
        "ESUS ELEKTRONIK SAN. VE DIS. TIC. LTD. STI." => Some("ESUS ELEKTRONIK"),
        "KOMSIS ELEKTRONIK SISTEMLERI SAN. TIC. LTD.STI" => Some("KOMSIS ELEKTRONIK SISTEMLERI"),
        "Hatel Elektronik LTD. STI." => Some("Hatel Elektronik"),
        "Signasystems Elektronik San. ve Tic. Ltd. Sti." => Some("Signasystems Elektronik"),
        "T PROJE MUHENDISLIK DIS TIC. LTD. STI." => Some("T PROJE MUHENDISLIK DIS TIC"),
        "Netlink Bilisim Sistemleri San. ve Tic. Ltd. Sti." => Some("Netlink Bilisim Sistemleri"),
        _ => None,
    }
}

/// Split a name into normalized lowercase word tokens (strips all punctuation).
fn normalize_words(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Trim known corporate suffixes from the end of an organization name.
/// Handles case variations, punctuation, multi-word suffixes, and UTF-8.
pub fn trim(name: &str) -> String {
    // Fix known data quality typos and collapse whitespace before matching
    // (double spaces also break exact hardcoded_trim key lookups).
    let name = normalize_spaces(&fix_name(name));

    // Check hardcoded full-name mappings (after typo fixes)
    if let Some(fixed) = hardcoded_trim(&name) {
        return normalize_spaces(fixed);
    }

    // Handle Japanese 株式会社 (Kabushiki Kaisha)
    let name = name
        .strip_suffix("株式会社")
        .map(|s| s.trim_end())
        .unwrap_or(&name);

    // Handle Turkish A.Ş. / A.Ş (Anonim Şirketi = joint-stock company).
    // Only the 'Ş' spellings are handled here: 'Ş' is non-ASCII and gets
    // dropped by the tokenizer, so the generic suffix list can't catch it.
    // The ASCII "A.S." is intentionally left to the "a s" suffix entry,
    // which also prevents a false match on the French "S.A.S.".
    let name = name
        .strip_suffix("A.Ş.")
        .or_else(|| name.strip_suffix("A.Ş"))
        .map(|s| s.trim_end())
        .unwrap_or(name);

    let words = normalize_words(name);
    if words.len() < 2 {
        return normalize_spaces(name.trim_end_matches(|c: char| {
            c == ',' || c == '.' || c.is_whitespace()
        }));
    }

    // Try matching from longest suffix first
    for suffix in CORP_SUFFIXES {
        // Split the same way as normalize_words so multi-word suffixes with
        // punctuation (e.g. "gmbh & co" -> ["gmbh", "co"]) actually match.
        let suffix_words: Vec<&str> = suffix
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
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
        // (skip trailing separators between keep words and suffix).
        let mut cut_pos = suffix_start;
        for (j, c) in name[last_keep_end..suffix_start].char_indices().rev() {
            if c.is_whitespace()
                || c == ','
                || c == '.'
                || c == ';'
                || c == '('
                || c == '&'
                || c == '-'
            {
                cut_pos = last_keep_end + j;
            } else {
                break;
            }
        }

        // If nothing remains, return original
        if cut_pos == 0 {
            return normalize_spaces(name);
        }

        let result = name[..cut_pos]
            .trim_end_matches(|c: char| c == ',' || c == '.' || c == '-' || c.is_whitespace())
            .to_string();

        // Re-balance parentheses: trimming a suffix that sat inside a
        // parenthetical (e.g. "ERI (EMPAC RESEARCH INC.)") also drops the
        // closing ')', leaving a dangling '('. Append the missing closers.
        let opens = result.matches('(').count();
        let closes = result.matches(')').count();
        if opens > closes {
            return format!("{}{}", result, ")".repeat(opens - closes));
        }

        return result;
    }

    normalize_spaces(name.trim_end_matches(|c: char| c == ',' || c == '.' || c.is_whitespace()))
}

/// Collapse multiple consecutive spaces into one.
fn normalize_spaces(s: &str) -> String {
    let mut result = s.to_string();
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    result
}