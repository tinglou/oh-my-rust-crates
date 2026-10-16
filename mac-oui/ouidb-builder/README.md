# ouidb-builder

Build tool for downloading and converting IEEE OUI (Organizationally Unique Identifier) data files.

## Features

- **Download** — Fetch the latest IEEE OUI CSV files (MA-L, MA-M, MA-S) from standards-oui.ieee.org
- **Convert** — Merge three CSVs into a single compact text file with country codes
- **Suffix Trimming** — Automatically strip common corporate suffixes (Inc., Ltd., GmbH, Co., Ltd., etc.)
- **Country Code Extraction** — Parse 2-letter country codes from organization addresses

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

Downloads three IEEE OUI CSV files into the `db/` directory:

| File | Source | Size |
|---|---|---|
| `db/oui.csv` | MA-L (24-bit) | ~3.8 MB |
| `db/iab.csv` | MA-M (28-bit) | ~380 KB |
| `db/oui36.csv` | MA-S (36-bit) | ~670 KB |

Also writes `db/timestamp.txt` with the current date (`YYYY-MM-DD`).

### `convert`

Parses the three CSV files and produces a unified output file.

**Default output** (`ieee-mac-oui.txt`): suffixes trimmed, ~52,000 entries

```
# Organizationally Unique Identifiers (OUIs)
# Original data comes from:
# MA-L(24 bits): https://standards-oui.ieee.org/oui/oui.csv
# MA-M(28 bits): https://standards-oui.ieee.org/iab/iab.csv
# MA-S(36 bits): https://standards-oui.ieee.org/oui36/oui36.csv
# Updated: 2026-10-16
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

## Output Format

Each line: `<MAC_prefix> <2-char_country> <organization_name>`

- **MAC prefix**: uppercase hex, 6/7/9 chars for 24/28/36-bit OUI
- **Country code**: 2-letter ISO code extracted from address (`US`, `CN`, `DE`, etc.), `--` if unavailable
- **Organization name**: company name with corporate suffixes stripped (default) or kept (`--keep-suffix`)

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
Done: 52145 entries written to ../macaddr-ouidb/ieee-mac-oui.txt
Trimmed: 38259/52145 (73.4%), unique: 20242/28138 (71.9%), saved: 136793 bytes
```

## Integration

The output file is consumed by the [`macaddr-ouidb`](../macaddr-ouidb) crate at compile time via its `build.rs`.

## Data Source

All data from [IEEE Standards Registration Authority](https://standards-oui.ieee.org/).

## License

Apache-2.0