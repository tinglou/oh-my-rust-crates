# conan2-rs

Rust library for integrating [Conan 2](https://docs.conan.io/2/) into Cargo build scripts.

## Usage

```rust
use conan2_rs::command::install::ConanInstall;
use conan2_rs::command::BuildPolicy;

// 1. Build conan install command
let command = ConanInstall::builder()?
    .profile("default")
    .build(BuildPolicy::Missing(None))
    .path("conanfile.txt")
    .into_command();

// 2. Run conan install, parse JSON output
let root = command.run()?;

// 3. Extract C++ build info for specific components
let cpp_info = root.get_cpp_info(&["hyperscan::hs", "hyperscan::hs_runtime"])?;

// 4. Emit cargo link directives
for libdir in &cpp_info.lib_dirs {
    println!("cargo:rustc-link-search={}", libdir);
}
for lib in &cpp_info.libs {
    println!("cargo:rustc-link-lib=static={}", lib);
}

// 5. Generate Rust FFI bindings from C header via bindgen
let bindings = bindgen::builder()
    .header("wrapper.h")
    .clang_arg(format!("-I{}", cpp_info.include_dirs[0]))
    .allowlist_function("my_.*")
    .allowlist_type("my_.*")
    .generate()?;
bindings.write_to_file("src/bindings.rs")?;
```

## API

| Module | Description |
|--------|-------------|
| `command::install` | `ConanInstall` builder → run conan → parse JSON |
| `command::types` | `BuildPolicy`, `ConanVerbosity` etc. |
| `model` | `ConanRoot`, `Graph`, `Recipe`, `CppInfo` — conan JSON schema types |
| `model::visit` | `get_cpp_info()`, `get_recipe_by_name()` — graph traversal |

## How it works

1. **Find conan** — via `$CONAN` env var or `which conan`
2. **Run** `conan install <conanfile> --format json --out-file ...`
3. **Parse** JSON output into `ConanRoot` (matches [Conan graph JSON schema](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html))
4. **Visit** the dependency graph to extract `CppInfo` per component
5. Caller uses `include_dirs` / `lib_dirs` / `libs` to configure the Rust build

## Conan JSON schema (partial)

```
ConanRoot
  └── graph: Graph
        ├── nodes: { id → Recipe }
        │     └── cpp_info: { component → CppInfo }
        │           ├── includedirs
        │           ├── libdirs
        │           └── libs
        └── root: { context → node_id }
```

## References

- [Conan Reference](https://docs.conan.io/2/reference.html)
- [Graph Info JSON Formatter](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)
