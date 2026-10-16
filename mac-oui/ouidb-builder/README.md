# ouidb-builder

Build tool for downloading and converting IEEE OUI (Organizationally Unique Identifier) data files.

## Features

- **Download** — Fetch the latest IEEE OUI CSV files (MA-L, MA-M, MA-S, IAB) from standards-oui.ieee.org
- **Convert** — Merge the CSVs into a single compact text file with country codes
- **Suffix Trimming** — Automatically strip common corporate suffixes (Inc., Ltd., GmbH, Co., Ltd., etc.)
- **Country Code Extraction** — Parse 2-letter country codes from organization addresses
- **Sanitization** — Remove invisible Unicode artifacts (NBSP, zero-width spaces, soft hyphens, …) found in the vendor-submitted IEEE data
- **De-duplication** — When IEEE reassigns a prefix, keep the most recent allocation

## Quick Start

```bash
# Download IEEE OUI data files
cargo run -p ouidb-builder -- download

# Convert to compact format (suffixes trimmed)
cargo run -p ouidb-builder -- convert

# Convert keeping full organization names
cargo run -p ouidb-builder -- convert --keep-suffix
```

## Commands

### `download`

Downloads four IEEE OUI CSV files into the `db/` directory:

| File | Source | Size |
|---|---|---|
| `db/oui.csv` | MA-L (24-bit) | ~3.8 MB |
| `db/mam.csv` | MA-M (28-bit) | ~700 KB |
| `db/oui36.csv` | MA-S (36-bit) | ~670 KB |
| `db/iab.csv` | IAB (36-bit) | ~380 KB |

Also writes `db/timestamp.txt` with the current date (`YYYY-MM-DD`).

### `convert`

Parses the CSV files and produces a unified output file.

**Default output** (`ieee-mac-oui.txt`): suffixes trimmed, ~58,000 entries

```
# Organizationally Unique Identifiers (OUIs)
# Original data comes from:
# MA-L(24 bits): https://standards-oui.ieee.org/oui/oui.csv
# MA-M(28 bits): https://standards-oui.ieee.org/oui28/mam.csv
# MA-S(36 bits): https://standards-oui.ieee.org/oui36/oui36.csv
# IAB(36 bits): https://standards-oui.ieee.org/iab/iab.csv
# Updated: 2026-10-16
# Duplicate prefixes are deduplicated: the LAST occurrence (most recent
# allocation) is kept. Dropped entries are logged to duplicate-prefix.log.
286FB9 CN Nokia Shanghai Bell
E80AB9 US Cisco Systems
F0EE7A US Apple
```

**With `--keep-suffix`** (`ieee-mac-oui-fullname.txt`): full names preserved

```
286FB9 CN Nokia Shanghai Bell Co., Ltd.
E80AB9 US Cisco Systems, Inc
F0EE7A US Apple, Inc.
```

Full names are kept as-is, applying only necessary normalizations: HTML entities (`&amp;` → `&`), fullwidth parentheses → ASCII, known spelling typos, and collapsed whitespace. No suffix trimming.

## Output Format

Each line: `<MAC_prefix> <2-char_country> <organization_name>`

- **MAC prefix**: uppercase hex, 6/7/9 chars for 24/28/36-bit OUI
- **Country code**: 2-letter ISO code extracted from address (`US`, `CN`, `DE`, etc.), `--` if unavailable
- **Organization name**: company name with corporate suffixes stripped (default) or kept with necessary normalizations only (`--keep-suffix`)

## Suffix Trimming

By default, `convert` strips common corporate legal suffixes from organization names. Supported suffixes include:

| Region | Suffixes |
|---|---|
| USA | Inc., Corp., Corporation, LLC, LLP, LP, PC, Incorporated |
| UK | Ltd., Limited, PLC |
| Germany | GmbH, AG, KG, GmbH & Co. KG |
| China | Co., Ltd., Corp., Ltd. |
| Japan | K.K., G.K. |
| Singapore | Pte. Ltd. |
| Australia | Pty. Ltd. |
| Netherlands | BV, NV |
| France/Spain | S.A., S.A.S., S.A.U. |
| Italy | S.p.A., S.r.l. |
| Nordic | A/S, AB, Oy, ApS |
| Other | ULC, s.r.o., Sdn Bhd |

Case-insensitive matching handles `Co.,LTD`, `co., ltd.`, `CO., LTD.`, etc.

**Statistics** (per run):
```
Done: 58779 entries written to ../macaddr-ouidb/ieee-mac-oui.txt (3 duplicate occurrences across 2 prefixes)
Trimmed: 44324/58782 (75.4%), unique: 25186/33227 (75.8%), saved: 173020 bytes
```

## Sanitization

IEEE CSV data is vendor-submitted and contains invisible Unicode artifacts. During conversion these are normalized/removed so the generated data (and the Rust source embedding it) stays clean:

- Unicode space separators (NBSP, en space, ideographic space, …) → ASCII space
- Soft hyphen (U+00AD), zero-width/bidi marks (U+200B..U+200F, U+202A..U+202E, U+2060..U+206F), BOM/ZWNBSP (U+FEFF), Arabic letter mark (U+061C) → removed

## De-duplication

IEEE reassigns prefixes over time, so the same prefix can appear multiple times across the source CSVs. `convert` keeps the **last** occurrence (the most recent allocation) and drops earlier ones. Every dropped entry is recorded in `duplicate-prefix.log`:

```
080030 (24 bits)
  kept:    080030 CH CERN
  dropped: 080030 US NETWORK RESEARCH CORPORATION
  dropped: 080030 AU ROYAL MELBOURNE INST OF TECH
```

## Integration

The output file is consumed by the [`macaddr-ouidb`](../macaddr-ouidb) crate at compile time via its `build.rs`.

## Data Source

All data from [IEEE Standards Registration Authority](https://standards-oui.ieee.org/).

## License

Apache-2.0