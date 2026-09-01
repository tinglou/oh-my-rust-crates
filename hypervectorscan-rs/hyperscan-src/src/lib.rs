//! hyperscan-src: 将 intel/hyperscan C++ 源码打包为 Rust crate，通过 cmake 从源码编译。
//!
//! # 设计原理
//!
//! 本 crate 使用 **openssl-src 模式**：cmake 调用放在 `lib.rs` 而非 `build.rs`。
//! 原因：将 cmake 调用放入 lib.rs 后，下游 crate（hypervectorscan-sys）在其 `build.rs`
//! 中通过函数调用执行。此时所有 `cargo:rustc-link-*` 指令都在 hypervectorscan-sys 的
//! build script 进程内发出 → Cargo 能正确将这些链接指令应用到最终产物。
//!
//! 如果放在本 crate 的 build.rs，link 指令会因为 crate 间隔离而无法传递到最终二进制。
//!
//! # 系统依赖
//!
//! 编译本 crate 需要：
//! - cmake ≥ 3.x
//! - C++ 编译器（gcc / clang）
//! - ragel
//! - boost-dev
//!
//! 这些依赖仅在启用 `vendored` feature 时需要。缺少时 cmake 的 configure 阶段会报错，
//! 错误信息会通过 cargo 输出清晰的提示。

use std::path::PathBuf;

/// 构建配置
pub struct Build {
    fat_runtime: bool,
    build_avx512: bool,
    build_avx512vbmi: bool,
}

/// 构建产物路径信息
pub struct Artifacts {
    /// 头文件目录（包含 hs/hs.h）
    pub include_dir: PathBuf,
    /// 库文件目录（libhs.a, libhs_runtime.a）
    pub lib_dir: PathBuf,
    /// 链接的库名列表
    pub libs: Vec<String>,
}

impl Build {
    /// 创建默认构建配置。
    ///
    /// 默认启用 FAT_RUNTIME（运行时 CPU 特性检测），避免在不支持 AVX-512 的 CPU 上
    /// 因预编译二进制包含 AVX-512 指令而导致 SIGILL。
    pub fn new() -> Self {
        Build {
            fat_runtime: true,
            build_avx512: false,
            build_avx512vbmi: false,
        }
    }

    /// 启用/禁用 FAT_RUNTIME（运行时 CPU 分发，默认启用）
    pub fn fat_runtime(&mut self, v: bool) -> &mut Self {
        self.fat_runtime = v;
        self
    }
    /// 启用 AVX512 指令集支持（x86_64 可选）
    pub fn avx512(&mut self, v: bool) -> &mut Self {
        self.build_avx512 = v;
        self
    }
    /// 启用 AVX512VBMI 指令集支持（x86_64 可选，依赖 BUILD_AVX512）
    pub fn avxvnni(&mut self, v: bool) -> &mut Self {
        self.build_avx512vbmi = v;
        self
    }

    /// 执行 cmake 构建，编译 Hyperscan 静态库。
    ///
    /// 此函数内部：
    /// 1. 调用 `cmake::Config::new("hyperscan")` 配置构建
    /// 2. 设置必要的 cmake flags（静态库、Release、FAT_RUNTIME 等）
    /// 3. 调用 `config.build()` 执行编译+安装
    /// 4. 通过 `println!("cargo:rustc-link-*")` 输出链接指令
    ///    （这些指令由调用者进程发出，因此能正确应用到最终二进制）
    /// 5. 返回包含头文件/库文件路径的 Artifacts
    pub fn build(&mut self) -> Artifacts {
        println!(
            "cargo:warning=Vendored Hyperscan build — requires: cmake, C++ compiler, ragel, boost-dev"
        );
        let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("hyperscan");
        let mut config = cmake::Config::new(source_dir);
        config
            .define("BUILD_STATIC_LIBS", "ON")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_INSTALL_LIBDIR", "lib")
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_BENCHMARKS", "OFF");

        if self.fat_runtime {
            config.define("FAT_RUNTIME", "ON");
        }
        if self.build_avx512 {
            config.define("BUILD_AVX512", "ON");
        }
        if self.build_avx512vbmi {
            config.define("BUILD_AVX512VBMI", "ON");
        }

        let dst = config.build();

        // Some cmake versions install to lib64 or multiarch paths;
        // search lib/, lib64/, lib/<arch>-linux-gnu/ — first containing libhs.a wins.
        let lib_dir = {
            let candidates: Vec<PathBuf> = vec![
                dst.join("lib"),
                dst.join("lib64"),
                dst.join(format!("lib/{}-linux-gnu", std::env::consts::ARCH)),
            ];
            candidates
                .into_iter()
                .find(|d| d.join("libhs.a").exists())
                .unwrap_or_else(|| dst.join("lib"))
        };

        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=hs");
        println!("cargo:rustc-link-lib=static=hs_runtime");

        Artifacts {
            include_dir: dst.join("include"),
            lib_dir,
            libs: vec!["hs".into(), "hs_runtime".into()],
        }
    }
}
