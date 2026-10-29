# cargo-undep

[中文说明](README-cn.md)

Find and remove unused `[workspace.dependencies]` from a Cargo workspace, and
hoist compatible member dependency sources into `[workspace.dependencies]`.

`cargo-undep` checks every workspace member for dependencies inherited with
`workspace = true`. A workspace dependency that no member inherits is reported
as unused. With `--fix`, it is removed from the root `Cargo.toml` together with
its directly attached comments.

With `--hoist`, the tool instead finds dependencies that can safely be declared
once in `[workspace.dependencies]` and inherited by their member crates.

## Installation

```text
cargo install cargo-undep
```

After installation the tool can be invoked as a Cargo subcommand:

```text
cargo undep
```

## Local development

To get the same effect as `cargo install cargo-undep` while editing the crate
locally, install the workspace member from its source directory:

```text
cargo install --path cargo-undep
```

From the workspace root, use the package path. Re-run with `--force` after
changing the code so the installed binary is replaced:

```text
cargo install --path cargo-undep --force
```

The binary is installed into Cargo's bin directory (`~/.cargo/bin` on Unix,
`%USERPROFILE%\.cargo\bin` on Windows), which is normally already on `PATH`.
After that `cargo undep` uses the locally built version.

For a one-off run without installing:

```text
# From the workspace root
cargo run -p cargo-undep -- --help

# From the cargo-undep directory
cargo run -- --help
```

Note that `cargo run` executes the binary directly; the `cargo undep`
subcommand form only works after the binary has been installed into `PATH`.

## Usage

```text
cargo undep [clean|hoist] [--fix] [--allow-dirty] [--manifest-path PATH]
```

| Option | Description |
|--------|-------------|
| `clean` | Report or remove unused `[workspace.dependencies]` |
| `hoist` | Report or move compatible member dependency sources into `[workspace.dependencies]` |
| `--fix` | Apply changes for the selected command |
| `--allow-dirty` | Allow `--fix` when the git work tree is dirty or git is unavailable |
| `--manifest-path PATH` | Path to the workspace root `Cargo.toml` |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

Without a command, the tool reports both unused workspace dependencies and hoistable member dependencies. `--fix` requires `clean` or `hoist`.

## Usage

### Report unused and hoistable dependencies

```text
$ cargo undep
Unused workspace dependencies (0):
No unused workspace dependencies found.
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - macaddr-ouidb-support: mixed default-features
  - serde: mixed default-features
  - thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep clean --fix` to remove unused dependencies or `cargo undep hoist --fix` to hoist dependencies.
```

### Report only unused dependencies

```text
$ cargo undep clean
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Run `cargo undep clean --fix` to remove them.
```

### Remove unused dependencies

```text
$ cargo undep clean --fix
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Removed 2 unused workspace dependencies from /path/to/Cargo.toml.
```

### Report hoistable dependencies

```text
$ cargo undep hoist
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - macaddr-ouidb-support: mixed default-features
  - serde: mixed default-features
  - thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep hoist --fix` to apply these changes.
```

### Hoist dependencies

```text
$ cargo undep hoist --fix
Hoistable workspace dependencies (24):
  ...

Updated 15 files and hoisted 24 workspace dependencies.
```

## Hoisting member dependencies

`--hoist` moves only the dependency source (`version`, `path`, `git`, and
related source keys) into `[workspace.dependencies]`. Attributes that belong to
the member stay in the member manifest, including `features`, `optional`, and
`default-features`:

```toml
# Member before
serde = { version = "1.0", features = ["derive"], optional = true }

# Member after
serde = { workspace = true, features = ["derive"], optional = true }
```

The workspace entry receives the shared source:

```toml
[workspace.dependencies]
serde = "1.0"
```

The tool is conservative:

- Compatible simple version requirements are merged by raising the lower bound,
  for example `1.0` and `1.2` become `1.2`.
- Incompatible major versions, different paths, different git sources, and
  different `package` aliases are skipped.
- Mixed `default-features` values are skipped, because Cargo cannot represent
  both values without editing member attributes.
- Custom registries are skipped.
- Dependencies already declared in `[workspace.dependencies]` are not modified.
- Target-specific dependency tables are supported.
- Comments and unrelated formatting are preserved through `toml_edit`.

Skipped dependencies are listed with the reason, but do not prevent other
dependencies from being hoisted.

## What counts as used

A `[workspace.dependencies]` entry is considered used when a workspace member
declares the same dependency key with `workspace = true` in any of these tables:

- `[dependencies]`
- `[dev-dependencies]` / `[dev_dependencies]`
- `[build-dependencies]` / `[build_dependencies]`
- `[target.'cfg(...)'.dependencies]`
- `[target.'cfg(...)'.dev-dependencies]`
- `[target.'cfg(...)'.build-dependencies]`

Both inline tables and `[dependencies.foo]` sub-tables are supported:

```toml
[dependencies]
serde = { workspace = true }
```

```toml
[dependencies.serde]
workspace = true
```

Cargo automatically includes in-workspace path dependencies as members, so
`cargo-undep` follows path dependencies as well. `[workspace].members` glob
patterns and `[workspace].exclude` are supported.

## Comment handling

When a dependency is removed, comments directly above it are removed too, as
long as they are contiguous and not separated by a blank line:

```toml
[workspace.dependencies]
# This comment is removed with the dependency.
unused = "1.0"

# Both comments below are attached to also-unused, so both are removed.
# The blank line above separates them from the previous entry.
also-unused = "2.0"

used = "1.0"
```

Commented-out dependencies are preserved:

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
unused = "1.0"
```

After `--fix`:

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
```

A line is treated as a commented-out dependency when it has the form
`# name = value`, where the key is a valid dependency name and the value looks
like a dependency value (a string, inline table, or bare number). Such a line
also acts as a boundary: comments above it are not removed.

## Git safety

`--fix` is destructive, so it requires a clean git work tree by default:

1. `git` must be available.
2. The workspace must be inside a git work tree.
3. `git status --porcelain` must be empty.

If any check fails, the tool refuses to modify files and prints an error.
When git is unavailable or the tree is dirty, pass `--allow-dirty` to proceed
explicitly. Untracked files also make the work tree dirty.

## Implementation notes

- [`toml_edit`](https://crates.io/crates/toml_edit) parses the manifests and
  provides byte spans for keys and values. It is the format-preserving TOML
  library used by `cargo-edit`.
- [`glob`](https://crates.io/crates/glob) expands `[workspace].members` patterns.
- Actual removal is performed on the original text by line range, preserving
  line endings (including CRLF), blank lines, and unrelated comments.
- git checks use `std::process::Command`; no extra git library is required.

## Limitations

- Only dependencies declared in workspace member manifests are considered.
- `[patch]`, `[replace]`, and references outside dependency tables are not
  treated as usage.
- The commented-out dependency heuristic is intentionally conservative: a
  comment such as `# note = "text"` is also preserved.
- Without `--hoist`, only the workspace root `Cargo.toml` is modified. Hoisting
  may also update member manifests.
- Hoisting does not modify existing `[workspace.dependencies]` entries and does
  not merge dependencies with incompatible sources or mixed `default-features`.

## Development

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Apache-2.0
