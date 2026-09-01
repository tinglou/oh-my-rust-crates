//! vectorscan-src: 将 VectorCamp/vectorscan C++ 源码打包为 Rust crate，通过 cmake 从源码编译。
//!
//! Vectorscan 是 Intel Hyperscan 的可移植分支，支持 ARM NEON/SVE2、Power VSX 等架构。
//! 在 ARM64 平台上，hypervectorscan-sys 使用本 crate 而非 hyperscan-src。
//!
//! # 设计原理
//!
//! 同 hyperscan-src：cmake 调用放在 lib.rs（非 build.rs），使 link 指令在下游 crate
//! 的 build script 进程内发出，保证正确传递到最终产物。
//!
//! # 系统依赖
//!
//! 编译本 crate 需要：
//! - cmake ≥ 3.x
//! - C++ 编译器（gcc / clang）
//! - ragel
//! - boost-dev
//!
//! SIMDe 后端（BUILD_SIMDE_BACKEND）默认关闭，不影响构建。

use std::path::PathBuf;

/// 构建配置
pub struct Build {
    fat_runtime: bool,
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
    /// 默认启用 FAT_RUNTIME（ARM ASIMD/SVE/SVE2 运行时分发），
    /// 禁用 USE_CPU_NATIVE（避免绑定到编译机器的 CPU 特性）。
    pub fn new() -> Self {
        Build { fat_runtime: true }
    }

    /// 启用/禁用 FAT_RUNTIME（运行时 CPU 分发，ARM 平台默认启用）
    pub fn fat_runtime(&mut self, v: bool) -> &mut Self {
        self.fat_runtime = v;
        self
    }

    /// 执行 cmake 构建，编译 Vectorscan 静态库。
    ///
    /// cmake flags：
    /// - BUILD_STATIC_LIBS=ON: 编译静态库
    /// - FAT_RUNTIME=ON: ARM 运行时 CPU 特性调度
    /// - USE_CPU_NATIVE=OFF: 不针对编译机器 CPU 优化（保证跨机型可移植）
    /// - BUILD_UNIT=OFF: 跳过单元测试编译
    /// - BUILD_SIMDE_BACKEND 保持默认 OFF（ARM 使用 NEON/SVE2 原生后端）
    pub fn build(&mut self) -> Artifacts {
        println!(
            "cargo:warning=Vendored Vectorscan build — requires: cmake, C++ compiler, ragel, boost-dev"
        );
        let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vectorscan");
        let mut config = cmake::Config::new(source_dir);
        config
            .define("BUILD_STATIC_LIBS", "ON")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_INSTALL_LIBDIR", "lib")
            .define("BUILD_UNIT", "OFF")
            .define("BUILD_TOOLS", "OFF")
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_DOC", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("USE_CPU_NATIVE", "OFF");

        if self.fat_runtime {
            config.define("FAT_RUNTIME", "ON");
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
