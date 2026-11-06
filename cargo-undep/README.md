# cargo-undep

[中文说明](README-cn.md)

Find and remove unused `[workspace.dependencies]` from a Cargo workspace, and hoist compatible member dependency sources into `[workspace.dependencies]`.

## Key capabilities

### `clean` — Find and remove unused workspace dependencies

Scans every workspace member for `workspace = true` inheritance. A workspace dependency that no member inherits is reported as unused. With `--fix`, it is removed from the root `Cargo.toml` together with its directly attached comments.

### `hoist` — Hoist member dependency sources into workspace dependencies

Finds dependencies that can safely be declared once in `[workspace.dependencies]` and inherited by their member crates. Moves only the source (`version`, `path`, `git`) to the workspace level; member-level attributes (`features`, `optional`, `default-features`) stay in the member crate. With `--fix`, also enforces inheritance for dependencies already declared in `[workspace.dependencies]`.

### `sort` — Group dependency tables by path and category

Sorts `[workspace.dependencies]` and every member's `[dependencies]` table when they contain at least 10 entries. Path dependencies come first, other dependencies are grouped by their crate's first category, and dependencies without a category go into a final `misc` group. A trailing comment such as `# category = network-programming` overrides all other classification. Groups are separated by comment lines and entries are sorted by name. Contiguous comments above a dependency and any trailing comment on its line move with the entry.

## Installation

```text
cargo install cargo-undep
```

After installation, invoke as a Cargo subcommand:

```text
cargo undep
```

## Quick start

```bash
# Report both unused and hoistable dependencies
cargo undep

# Check unused workspace dependencies only
cargo undep clean

# Remove unused workspace dependencies
cargo undep clean --fix

# Check hoistable member dependencies
cargo undep hoist

# Hoist member dependencies (also enforces inheritance for already-declared deps)
cargo undep hoist --fix

# Check dependency tables that would be sorted
cargo undep sort

# Sort dependency tables
cargo undep sort --fix
```

## Command line options

```bash
cargo undep [clean|hoist|sort] [--fix] [--build] [--allow-dirty] [--manifest-path PATH]
```

| Option | Description |
|--------|-------------|
| `clean` | Report or remove unused `[workspace.dependencies]` |
| `hoist` | Report or move compatible member dependency sources into `[workspace.dependencies]` |
| `sort` | Report or sort `[workspace.dependencies]` and member `[dependencies]` by path and category |
| `--fix` | Apply changes for the selected command |
| `--build` | After `--fix` with changes, run `cargo build --all-targets` to verify |
| `--allow-dirty` | Allow `--fix` when the git work tree is dirty or git is unavailable |
| `--manifest-path PATH` | Path to the workspace root `Cargo.toml` |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

Without a command, the tool reports both unused workspace dependencies and hoistable member dependencies. `--fix` requires `clean`, `hoist`, or `sort`. When run from a member directory, `hoist` only processes crates under the current directory.

## Usage examples

### Report both unused and hoistable dependencies

```text
$ cargo undep
No unused workspace dependencies found.
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - [default-features conflict] macaddr-ouidb-support: mixed default-features
  - [default-features conflict] serde: mixed default-features
  - [version conflict] thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep clean --fix` to remove unused dependencies or `cargo undep hoist --fix` to hoist dependencies.
```

### Check and remove unused workspace dependencies

```text
$ cargo undep clean
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Run `cargo undep clean --fix` to remove them.

$ cargo undep clean --fix
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Removed 2 unused workspace dependencies from /path/to/Cargo.toml.
```

### Check and hoist member dependencies

```text
$ cargo undep hoist
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - [default-features conflict] macaddr-ouidb-support: mixed default-features
  - [default-features conflict] serde: mixed default-features
  - [version conflict] thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep hoist --fix` to apply these changes.

$ cargo undep hoist --fix
Hoistable workspace dependencies (24):
  ...

Updated 15 files and hoisted 24 workspace dependencies.
```

### Sort dependency tables

```text
$ cargo undep sort
Cargo.toml files to update (2):
  - /path/to/Cargo.toml
  - /path/to/member/Cargo.toml

Run `cargo undep sort --fix` to apply these changes.

$ cargo undep sort --fix
Cargo.toml files to update (2):
  - /path/to/Cargo.toml
  - /path/to/member/Cargo.toml

Updated 2 Cargo.toml files.
```

### Sorting transforms tables like this

```toml
# Before
[dependencies]
zeta = { package = "serde_json", version = "1" }
# comment for anyhow
anyhow = "1" # trailing comment
local = { path = "../local" }
clap = "4"
...

# After
[dependencies]
# --- path ---
local = { path = "../local" }
# --- command-line-interface ---
clap = "4"
# --- encoding ---
zeta = { package = "serde_json", version = "1" }
# --- rust-patterns ---
# comment for anyhow
anyhow = "1" # trailing comment
```

### Hoisting transforms member dependencies like this

```toml
# Member before
serde = { version = "1.0", features = ["derive"], optional = true }

# Member after (only source moved, attributes preserved)
serde = { workspace = true, features = ["derive"], optional = true }
```

```toml
# Workspace Cargo.toml receives the shared source
[workspace.dependencies]
serde = "1.0"
```

## Strategies

### Reference detection

A `[workspace.dependencies]` entry is considered used when a workspace member declares the same dependency key with `workspace = true` in any of these tables:

- `[dependencies]`
- `[dev-dependencies]` / `[dev_dependencies]`
- `[build-dependencies]` / `[build_dependencies]`
- `[target.'cfg(...)'.dependencies]`
- `[target.'cfg(...)'.dev-dependencies]`
- `[target.'cfg(...)'.build-dependencies]`

Both inline tables and `[dependencies.foo]` sub-tables are supported. Path dependencies within the workspace are automatically followed. `[workspace].members` glob patterns and `[workspace].exclude` are respected.

### Dependency sorting

`sort` classifies each dependency in `[workspace.dependencies]` and member `[dependencies]` tables:

1. A category in the dependency's trailing comment takes priority over everything else: `snmp2 = { version = "0.5.3" } # category = network-programming`.
2. Dependencies with a `path` source go into the first `path` group.
3. Other dependencies use the first category from `cargo metadata`, normalized to the segment before `::`. For example, `development-tools::profiling` becomes `development-tools`.
4. Dependencies without a category go into the final `misc` group.

Groups are ordered as `path`, then categories alphabetically, then `misc`. Within each group, dependencies are sorted by name. A table with fewer than 10 entries is left unchanged.

Each group is preceded by a comment line:

```toml
[dependencies]
# --- path ---
local-helper = { path = "../local-helper" }
# --- encoding ---
serde = "1"
# --- misc ---
unknown-crate = "1"
```

The original declaration text is preserved while sorting. Contiguous comment lines directly above a dependency and any trailing comment on the same line move with that dependency.

### Comment handling

When a dependency is removed, comments directly above it are removed too, as long as they are contiguous and not separated by a blank line:

```toml
[workspace.dependencies]
# This comment is removed with the dependency.
unused = "1.0"

# Both comments below are attached to also-unused, so both are removed.
# The blank line above separates them from the previous entry.
also-unused = "2.0"

used = "1.0"
```

Commented-out dependencies are preserved and act as boundaries:

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
unused = "1.0"
```

After `--fix`, the commented-out dependency remains:

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
```

A line is treated as a commented-out dependency when it has the form `# name = value`, where the key is a valid dependency name and the value looks like a dependency value (string, inline table, or bare number). Such a line also acts as a boundary: comments above it are not removed.

### Version compatibility

Compatible simple version requirements are merged by raising the lower bound (caret semantics):

- `1.0` + `1.2` → `1.2`
- `0.2` + `0.2.5` → `0.2.5`
- `1.0` + `2.0` → **skipped** (incompatible major)
- `0.2` + `0.3` → **skipped** (incompatible minor in 0.x)
- Mixed `default-features` → **skipped** (cannot represent both without editing member attributes)

Skipped dependencies are listed with the reason, but do not prevent other dependencies from being hoisted.

### Git safety

`clean --fix` and `hoist --fix` are destructive, so they require a clean git work tree by default:

1. `git` must be available.
2. The workspace must be inside a git work tree.
3. `git status --porcelain` must be empty.

`sort --fix` only rewrites `Cargo.toml` files, so it only refuses when a `Cargo.toml` has uncommitted changes; unrelated dirty files are allowed.

If any check fails, the tool refuses to modify files and prints an error. When git is unavailable or the relevant files are dirty, pass `--allow-dirty` to proceed explicitly. Untracked files count as changes.

## License

Apache-2.0
