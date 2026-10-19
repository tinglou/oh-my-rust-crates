//! hypervectorscan-sys 构建脚本
//!
//! # 架构概述
//!
//! 本 build.rs 支持五种构建模式，通过 Cargo feature 切换：
//!
//! ## autobuild 模式（默认，feature = "autobuild"）
//!
//! 自动选择最优构建方式，优先级从高到低：
//! 1. conan2   — conan CLI 包管理器
//! 2. vcpkg    — vcpkg 包管理器
//! 3. system   — 系统已安装的库（pkg-config，如 apt install libhyperscan-dev）
//! 4. vendored — bundled 源码 cmake 编译
//!
//! ## 显式指定模式
//! - conan2:   强制 Conan
//! - vcpkg:    强制 vcpkg
//! - system:   强制使用系统已安装的库
//! - vendored: 强制 bundled 源码编译（需 cmake、ragel、boost-dev）
//!
//! # 平台与后端选择
//!
//! | 架构            | conan2/vcpkg/system 后端 | vendored 后端   |
//! |-----------------|--------------------------|-----------------|
//! | x86_64          | Hyperscan                | hyperscan-src   |
//! | aarch64/arm64   | Vectorscan               | vectorscan-src  |
//! | ppc64/riscv/其他| ❌                       | vectorscan-src  |
//!
//! # 关键设计
//!
//! vendored 构建的 cmake 调用在 *-src/src/lib.rs 中（非 build.rs）。
//! cargo:rustc-link-* 指令在**本进程内**发出 → 正确传递到最终产物。

#[cfg(any(
    feature = "autobuild",
    feature = "conan2",
    feature = "vcpkg",
    feature = "system",
    feature = "vendored",
))]
use std::path::Path;

// ───────────── 入口分发 ─────────────

#[cfg(feature = "autobuild")]
fn main() -> anyhow::Result<()> {
    autobuild()
}

#[cfg(all(feature = "conan2", not(feature = "autobuild")))]
fn main() -> anyhow::Result<()> {
    conan2_build()
}

#[cfg(all(feature = "vcpkg", not(feature = "autobuild")))]
fn main() -> anyhow::Result<()> {
    vcpkg_build()
}

#[cfg(all(feature = "system", not(feature = "autobuild")))]
fn main() -> anyhow::Result<()> {
    system_build()
}

#[cfg(all(feature = "vendored", not(feature = "autobuild")))]
fn main() -> anyhow::Result<()> {
    vendored_build()
}

#[cfg(not(any(
    feature = "autobuild",
    feature = "conan2",
    feature = "vcpkg",
    feature = "system",
    feature = "vendored",
)))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!(
        "hypervectorscan-sys: no build mode feature enabled.\n\
         Enable one of: autobuild, conan2, vcpkg, system, vendored\n\
         \n\
         Recommended usage (via hypervectorscan top-level crate, default autobuild):\n\
           hypervectorscan = \"0.1\"\n\
         \n\
         Or explicitly specify vendored mode:\n\
           hypervectorscan = {{ default-features = false, features = [\"vendored\"] }}"
    );
}

// ── autobuild：四级自动回退 ────────────────────────────────────────────

#[cfg(feature = "autobuild")]
fn autobuild() -> anyhow::Result<()> {
    if conan_available() {
        println!("cargo:warning=autobuild: conan detected, using conan2 build");
        return conan2_build();
    }
    if vcpkg_available() {
        println!("cargo:warning=autobuild: vcpkg detected, using vcpkg build");
        return vcpkg_build();
    }
    if system_available() {
        println!("cargo:warning=autobuild: system library detected, using system build");
        return system_build();
    }
    println!(
        "cargo:warning=autobuild: falling back to vendored build (bundled source cmake compile)"
    );
    vendored_build()
}

// ── 工具检测 ──────────────────────────────────────────────────────────

#[cfg(any(feature = "autobuild", feature = "conan2"))]
fn conan_available() -> bool {
    std::process::Command::new("conan")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(any(feature = "autobuild", feature = "vcpkg"))]
fn vcpkg_available() -> bool {
    std::process::Command::new("vcpkg")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// 检测系统是否通过包管理器安装了 hyperscan/vectorscan 及其开发头文件
/// 通过 pkg-config 检查。Windows 上 pkg-config 不存在，直接返回 false。
#[cfg(any(feature = "autobuild", feature = "system"))]
fn system_available() -> bool {
    // Windows 不支持 pkg-config（除 MSYS2 外），交给 vcpkg 处理
    if cfg!(target_os = "windows") {
        return false;
    }
    let target = std::env::var("TARGET").unwrap_or_default();
    let pc = if is_x86_64(&target) {
        "libhs"
    } else {
        "vectorscan"
    };
    std::process::Command::new("pkg-config")
        .args(["--exists", pc])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── system 构建：使用系统已安装的库（pkg-config 检测）──────────────────

/// 通过 pkg-config 查找系统已安装的 hyperscan/vectorscan
/// 适用场景：用户已通过 apt/yum/brew 安装好 dev 包
#[cfg(any(feature = "system", feature = "autobuild"))]
fn system_build() -> anyhow::Result<()> {
    let target = std::env::var("TARGET").unwrap_or_default();
    let package = if is_x86_64(&target) {
        "libhs"
    } else {
        "vectorscan"
    };
    println!("cargo:warning=system: looking up {package} via pkg-config");

    // 优先链接静态库
    let lib = pkg_config::Config::new()
        .statik(true)
        .probe(package)
        .map_err(|_| {
            anyhow::anyhow!(
                "pkg-config: {package} not found, please install the corresponding dev package"
            )
        })?;

    let include_paths: Vec<&Path> = lib.include_paths.iter().map(|p| p.as_path()).collect();
    generate_bindings(&include_paths, "wrapper.h")?;

    Ok(())
}

// ── vendored 构建：bundled 源码 cmake 编译 ─────────────────────────────

#[cfg(any(feature = "vendored", feature = "autobuild"))]
fn vendored_build() -> anyhow::Result<()> {
    println!("cargo:warning=vendored: compiling from bundled source (no external package manager)");

    let target = std::env::var("TARGET").unwrap_or_default();

    if is_x86_64(&target) {
        println!("cargo:warning=vendored backend: hyperscan-src (x86_64)");
        let mut build = hyperscan_src::Build::new();
        #[cfg(feature = "avx512")]
        build.avx512(true);
        #[cfg(feature = "avxvnni")]
        build.avxvnni(true);
        let artifacts = build.build();
        generate_bindings(&[&artifacts.include_dir], "wrapper.h")?;
    } else {
        println!("cargo:warning=vendored backend: vectorscan-src (non-x86_64)");
        let artifacts = vectorscan_src::Build::new().build();
        generate_bindings(&[&artifacts.include_dir], "wrapper.h")?;
    }

    Ok(())
}

#[cfg(any(
    feature = "autobuild",
    feature = "conan2",
    feature = "vcpkg",
    feature = "system",
    feature = "vendored",
))]
fn is_x86_64(target: &str) -> bool {
    target.contains("x86_64")
}

// ── vcpkg 构建 ─────────────────────────────────────────────────────────

#[cfg(any(feature = "vcpkg", feature = "autobuild"))]
fn vcpkg_build() -> anyhow::Result<()> {
    println!("cargo:warning=vcpkg build mode");

    let target = std::env::var("TARGET").unwrap_or_default();
    let package = if is_x86_64(&target) {
        "hyperscan"
    } else {
        "vectorscan"
    };
    println!("cargo:warning=vcpkg: looking for package {package}");

    let lib = match vcpkg::find_package(package) {
        Ok(lib) => lib,
        Err(_) => {
            println!("cargo:warning=vcpkg: {package} not found, running vcpkg install...");
            vcpkg_install(package)?;
            vcpkg::find_package(package)?
        }
    };

    let include_paths: Vec<&Path> = lib.include_paths.iter().map(|p| p.as_path()).collect();
    generate_bindings(&include_paths, "wrapper.h")?;

    Ok(())
}

#[cfg(any(feature = "vcpkg", feature = "autobuild"))]
fn vcpkg_install(package: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new("vcpkg")
        .args(["install", package])
        .status()?;
    anyhow::ensure!(status.success(), "vcpkg install {package} failed");
    Ok(())
}

// ── conan2 构建 ────────────────────────────────────────────────────────

#[cfg(any(feature = "conan2", feature = "autobuild"))]
use conan2_rs::command::BuildPolicy;
#[cfg(any(feature = "conan2", feature = "autobuild"))]
use conan2_rs::command::install::ConanInstall;

#[cfg(any(feature = "conan2", feature = "autobuild"))]
struct Backend {
    conanfile: &'static str,
    cpp_info_deps: &'static [&'static str],
    options: &'static [&'static str],
    force_build: Option<&'static str>,
}

#[cfg(any(feature = "conan2", feature = "autobuild"))]
fn detect_backend() -> anyhow::Result<Backend> {
    let target = std::env::var("TARGET").unwrap_or_default();

    let os = if target.contains("linux") {
        "linux"
    } else if target.contains("windows") {
        "windows"
    } else if target.contains("apple-darwin") {
        "macos"
    } else {
        anyhow::bail!("conan2: unsupported target platform: {target}")
    };

    let arch = if target.contains("aarch64") || target.contains("arm64") {
        "arm64"
    } else if target.contains("x86_64") {
        "x86_64"
    } else {
        anyhow::bail!("conan2: unsupported target architecture: {target}")
    };

    match (os, arch) {
        ("linux", "x86_64") => Ok(Backend {
            conanfile: "conanfile-hyperscan.txt",
            cpp_info_deps: &["hyperscan::hs", "hyperscan::hs_runtime"],
            options: &["hyperscan/*:fat_runtime=True"],
            force_build: Some("hyperscan*"),
        }),
        ("windows", "x86_64") | ("macos", "x86_64") => Ok(Backend {
            conanfile: "conanfile-hyperscan.txt",
            cpp_info_deps: &["hyperscan::hs", "hyperscan::hs_runtime"],
            options: &[],
            force_build: None,
        }),
        ("linux", "arm64") | ("macos", "arm64") => Ok(Backend {
            conanfile: "conanfile-vectorscan.txt",
            cpp_info_deps: &["vectorscan::hs", "vectorscan::hs_runtime"],
            options: &[
                "vectorscan/*:with_fat_runtime=True",
                "vectorscan/*:with_cpu_native=False",
                "vectorscan/*:shared=False",
            ],
            force_build: Some("vectorscan*"),
        }),
        _ => anyhow::bail!("conan2: unsupported target: {os}/{arch}"),
    }
}

#[cfg(any(feature = "conan2", feature = "autobuild"))]
fn conan2_build() -> anyhow::Result<()> {
    let backend = detect_backend()?;
    println!(
        "cargo:warning=conan2 backend: conanfile={}, force_build={:?}",
        backend.conanfile, backend.force_build
    );
    println!("cargo:rerun-if-env-changed=TARGET");

    let build_policy = match backend.force_build {
        Some(pattern) => BuildPolicy::Pattern(pattern.to_string()),
        None => BuildPolicy::Missing(None),
    };

    let mut builder = ConanInstall::builder()?
        .profile("default")
        .build(build_policy)
        .path(backend.conanfile);

    for opt in backend.options {
        builder = builder.options(*opt);
    }

    let command = builder.into_command();
    let root = command.run()?;

    let cpp_info = root.get_cpp_info(backend.cpp_info_deps)?;

    for libdir in cpp_info.lib_dirs.iter() {
        println!("cargo:rustc-link-search={}", libdir);
    }
    for lib in cpp_info.libs.iter() {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    let include_dirs: Vec<String> = cpp_info.include_dirs.iter().cloned().collect();
    let include_paths: Vec<&Path> = include_dirs.iter().map(|d| Path::new(d)).collect();
    generate_bindings(&include_paths, "wrapper.h")?;

    Ok(())
}

// ── 共享 ──────────────────────────────────────────────────────────────

/// bindgen 从 hs/hs.h 生成 Rust FFI 绑定 → OUT_DIR/hyperscan.rs
#[cfg(any(
    feature = "autobuild",
    feature = "conan2",
    feature = "vcpkg",
    feature = "system",
    feature = "vendored",
))]
fn generate_bindings(include_dirs: &[&Path], header_file: &str) -> anyhow::Result<()> {
    let out_dir = std::env::var("OUT_DIR")?;
    let out_path = format!("{out_dir}/hyperscan.rs");
    println!("cargo:rerun-if-changed={header_file}");

    let mut bindgen = bindgen::builder();
    for include in include_dirs {
        bindgen = bindgen.clang_arg(format!("-I{}", include.display()));
    }
    let bindgen = bindgen.header(header_file);

    let bindgen = bindgen
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_partialeq(true)
        .allowlist_function("hs_.*")
        .allowlist_type("hs_.*")
        .allowlist_var("HS_.*");
    let bindings = bindgen.generate()?;
    bindings.write_to_file(out_path)?;

    Ok(())
}
