//!  Conan json 格式的解析结构体定义
//! [Json schema](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)
//! The output shows the graph information processed by Conan in each command.
//!
//! **Note**
//!
//! > This feature is in **preview**. It means that it is very unlikely to be removed and unlikely to have breaking changes. Maintainers will try as much as possible to not break it, and only do it if very necessary. See [the Conan stability](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html../../../introduction.html#stability) section for more information.
//!

use std::collections::HashMap;

use serde::Deserialize;

/// 根结构，包含整个 Conan 构建的依赖图及相关元数据。
///
/// Root structure for conanbuildinfo.json, contains the full dependency graph and related metadata for the Conan build.
/// [Json schema](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)
///
#[derive(Debug, Deserialize)]
pub struct ConanRoot {
    /// Conan 构建的依赖图，包含所有节点（包/组件）及其关系。
    ///
    /// The dependency graph of the Conan build, including all nodes (packages/components) and their relationships.
    pub graph: Graph,
}

/// Conan 依赖图结构，包含所有节点、根节点及依赖解析相关信息。
///
/// The dependency graph, including all nodes, root nodes, and dependency resolution information.
#[derive(Debug, Deserialize)]
pub struct Graph {
    /// 依赖图中的所有节点（包/组件），以节点 id 作为 key。
    ///
    /// All nodes (packages/components) in the dependency graph, keyed by node id.
    pub nodes: HashMap<String, Recipe>,
    /// 依赖图的根节点（通常为主包），key 为 context（如 host/build），value 为节点 id。
    ///
    /// The root node(s) of the graph, key is context (e.g. host/build), value is node id.
    pub root: HashMap<String, String>,
    /// 被覆盖的依赖项（overrides），key 为原始依赖，value 为覆盖后的依赖。
    ///
    /// Overridden requirements in the graph, key is original requirement, value is the override.
    #[serde(default)]
    pub overrides: HashMap<String, String>,
    /// 依赖项的已解析版本范围，key 为依赖名，value 为具体版本。
    ///
    /// Resolved version ranges for requirements, key is requirement name, value is resolved version.
    #[serde(default)]
    pub resolved_ranges: HashMap<String, String>,
    /// 被替换的依赖项，key 为原始依赖，value 为替换后的依赖。
    ///
    /// Requirements that have been replaced in the graph, key is original, value is replacement.
    #[serde(default)]
    pub replaced_requires: HashMap<String, String>,
    /// 依赖图解析失败时的错误信息。
    ///
    /// Error message if the graph resolution failed.
    pub error: Option<String>,
}

/// Conan 依赖图中的节点，表示一个包或组件，包含包的元数据、依赖、构建信息等。
/// 每个 Conan 包都需要有一个对应的 recipe (conanfile.py) 来描述如何构建该包。这包括但不限于：
/// * 包的基本信息（名称、版本等）
/// * 包的依赖关系
/// * 获取源代码的方法
/// * 如何配置和执行构建命令
/// * 安装步骤
/// * 打包步骤
/// * Recipes 还可以包含条件逻辑，以便于适应不同的操作系统、架构或编译器版本等。
///
/// A node in the Conan dependency graph, representing a package or component, including metadata, dependencies, and build info.
/// 引文：
/// - [Conan Reference](https://docs.conan.io/2/reference/conanfile/attributes.html)
/// - [Conan dependency graph node](https://github.com/conan-io/conan/blob/develop/conans/model/graph.py)
#[derive(Debug, Deserialize)]
pub struct Recipe {
    /// 节点 id（在依赖图中唯一）。
    ///
    /// Node id (unique within the graph).
    pub id: String,

    /// Conan 包引用字符串（如 "pkg/1.0@user/channel"）。
    ///
    /// Conan reference string (e.g. "pkg/1.0@user/channel").
    #[serde(rename = "ref")]
    pub ref_: Option<String>,

    #[serde(flatten)]
    pub package_reference: PackageReference,

    #[serde(flatten)]
    pub metadata: Metadata,

    #[serde(flatten)]
    pub requirements: Requirements,

    #[serde(flatten)]
    pub binary_model: BinaryModel,

    #[serde(flatten)]
    pub build: Build,

    #[serde(flatten)]
    pub folder_layout: FolderAndLayout,

    #[serde(flatten)]
    pub consumer_info: PackageInformationConsumer,

    #[serde(flatten)]
    pub other: RecipeOther,

    /// 配方类型（如 "Cache"、"Editable"），表示包的来源或状态。
    ///
    /// Recipe type (e.g. "Cache", "Editable"), indicates the source or state of the package.
    pub recipe: Option<String>,

    /// 上一个包修订号（package revision）。
    ///
    /// Previous package revision (prev).
    pub prev: Option<String>,
    /// 配方修订号（recipe revision）。
    ///
    /// Recipe revision (rrev).
    pub rrev: Option<String>,
    /// 配方修订时间戳。
    ///
    /// Recipe revision timestamp.
    pub rrev_timestamp: Option<f64>,
    /// 包修订时间戳。
    ///
    /// Package revision timestamp.
    pub prev_timestamp: Option<f64>,
    /// 配方所在的远程仓库。
    ///
    /// Remote where the recipe was found.
    pub remote: Option<String>,
    /// 二进制包所在的远程仓库。
    ///
    /// Remote where the binary was found.
    pub binary_remote: Option<String>,
    /// build-requires 的 build id。
    ///
    /// Build id for build-requires.
    pub build_id: Option<String>,
    /// 二进制包状态（如 "Cache"、"Build"、"Download"）。
    ///
    /// Binary status (e.g. "Cache", "Build", "Download").
    pub binary: Option<String>,
    /// 构建是否无效。
    ///
    /// Whether the build is invalid.
    pub invalid_build: Option<bool>,
    /// 构建无效的原因。
    ///
    /// Reason for invalid build.
    pub info_invalid: Option<String>,

    /// 节点标签。
    ///
    /// Label for this node.
    pub label: Option<String>,
    /// 是否为 vendor 包。
    ///
    /// Whether this node is a vendor package.
    pub vendor: Option<bool>,

    /// 节点上下文（如 host/build/test）。
    ///
    /// Context for this node (e.g. host/build/test).
    pub context: Option<String>,
}

/// Recipe attributes that can define the main pkg/version@user/channel package reference.
#[derive(Debug, Deserialize)]
pub struct PackageReference {
    /// 包名。
    ///
    /// Package name.
    pub name: Option<String>,

    /// 包版本号。
    ///
    /// Package version.
    pub version: Option<String>,

    /// Conan reference 的 user 部分。
    ///
    /// User part of the Conan reference.
    pub user: Option<String>,

    /// Conan reference 的 channel 部分。
    ///
    /// Channel part of the Conan reference.
    pub channel: Option<String>,

    /// 是否为测试包。
    ///
    /// Whether this node is a test package.
    pub test: Option<bool>,
}

/// Optional metadata, like license, description, author, etc.
/// Not necessary for most cases, but can be useful to have.
#[derive(Debug, Deserialize)]
pub struct Metadata {
    /// 包描述。
    ///
    /// Description string.
    pub description: Option<String>,

    /// 许可证信息。
    ///
    /// License string.
    pub license: Option<String>,

    /// 作者信息。
    ///
    /// Author string.
    pub author: Option<String>,
    /// 主题标签列表（topics）。
    ///
    /// List of topics.
    pub topics: Option<Vec<String>>,

    /// 主页 URL。
    ///
    /// Homepage URL.
    pub homepage: Option<String>,

    /// 项目主页 URL。
    ///
    /// Project URL.
    pub url: Option<String>,
}

/// Attribute form of the dependencies simple declarations, like requires, tool_requires.
/// For more advanced way to define requirements, use the requirements(), build_requirements()
/// methods instead.
#[derive(Debug, Deserialize)]
pub struct Requirements {
    /// Python 依赖项（python_requires），key 为依赖名。
    ///
    /// Python requirements for this node, key is requirement name.
    pub python_requires: Option<HashMap<String, Vec<ValueVariant>>>,
    /// 系统依赖项，key 为依赖名。
    ///
    /// System requirements for this node, key is requirement name.
    pub system_requires: HashMap<String, ValueVariant>,
}

#[derive(Debug, Deserialize)]
pub struct BinaryModel {
    /// 包类型（如 "library"、"application"）。
    ///
    /// Package type (e.g. "library", "application").
    pub package_type: Option<String>,

    /// 构建设置（如 os、arch、compiler 等）。
    ///
    /// Build settings (e.g. os, arch, compiler, etc.).
    pub settings: Settings,

    /// 该节点的选项值，key 为选项名。
    ///
    /// Option values for this node, key is option name.
    pub options: HashMap<String, ValueVariant>,

    /// 包的默认选项（如 shared、fPIC 等），key 为选项名。
    ///
    /// Default options for the package, key is option name.
    pub default_options: Option<HashMap<String, ValueVariant>>,

    /// 选项描述。
    ///
    /// Description of options.
    pub options_description: Option<String>,

    /// 该节点的选项定义，key 为选项名，value 为可选值列表。
    ///
    /// Option definitions for this node, key is option name, value is list of possible values.
    pub options_definitions: HashMap<String, Vec<ValueVariant>>,

    /// 支持的编程语言列表。
    ///
    /// Supported languages.
    pub languages: Option<Vec<String>>,

    /// 附加节点信息（settings、options、兼容性等）。
    ///
    /// Additional node info (settings, options, compatibility, etc.).
    pub info: Option<NodeInfo>,

    /// Conan 包 id，唯一标识二进制包。
    ///
    /// Conan package id, uniquely identifies the binary package.
    pub package_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Build {
    /// 使用的生成器列表（如 cmake、make 等）。
    ///
    /// List of generators used (e.g. cmake, make, etc.).
    pub generators: Option<Vec<String>>,

    /// 构建策略（如 "missing"，控制缺失包的构建行为）。
    ///
    /// Build policy (e.g. "missing"), controls build behavior for missing packages.
    pub build_policy: Option<String>,
    /// 构建时是否使用 Windows bash。
    ///
    /// Use Windows bash for build.
    pub win_bash: Option<bool>,
    /// 运行时是否使用 Windows bash。
    ///
    /// Use Windows bash for run.
    pub win_bash_run: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FolderAndLayout {
    /// 源码文件夹路径。
    ///
    /// Path to the source folder.
    pub source_folder: Option<String>,

    /// 构建文件夹路径。
    ///
    /// Path to the build folder.
    pub build_folder: Option<String>,
    /// 包文件夹路径。
    ///
    /// Path to the package folder.
    pub package_folder: Option<String>,

    /// 配方文件夹路径。
    ///
    /// Path to the recipe folder.
    pub recipe_folder: Option<String>,

    /// 生成器文件夹路径。
    ///
    /// Path to the generators folder.
    pub generators_folder: Option<String>,

    /// 不可变包文件夹路径。
    ///
    /// Path to the immutable package folder.
    pub immutable_package_folder: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PackageInformationConsumer {
    /// C++ 构建信息（每个组件），key 为组件名。
    ///
    /// C++ build information for this node (per component), key is component name.
    pub cpp_info: HashMap<String, CppInfo>,

    /// 节点的配置信息（conf），key 为配置项。
    ///
    /// Configuration information for this node, key is config item.
    pub conf_info: HashMap<String, ValueVariant>,

    /// 弃用信息。
    ///
    /// Deprecation message.
    pub deprecated: Option<String>,
    /// 提供的虚拟包。
    ///
    /// Provided virtual packages.
    pub provides: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecipeOther {
    /// 节点的依赖项，key 为依赖名。
    ///
    /// Dependencies of this node, key is dependency name.
    pub dependencies: HashMap<String, Dependency>,
    /// 修订模式（如 "scm"，源码管理相关）。
    ///
    /// Revision mode (e.g. "scm"), related to source control management.
    pub revision_mode: Option<String>,
    /// 上传策略。
    ///
    /// Upload policy.
    pub upload_policy: Option<String>,
}

/// Conan JSON 字段的变体类型，支持 string、bool、float、null 等多种类型。
///
/// Variant type for values in Conan JSON (string, bool, float, or none/null).
/// Values for each option can be typed or plain strings ("value", True, 42,…).
/// There are two special values:
/// None: Allow the option to have a None value (not specified) without erroring.
// "ANY": For options that can take any value, not restricted to a set.
/// 引文：
/// - [Conan JSON value types](https://docs.conan.io/2/reference/conanfile/attributes.html#options)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueVariant {
    /// 空值（null/None）。
    ///
    /// Null/None value.
    None,
    /// 字符串类型。
    ///
    /// String value.
    Str(String),
    /// 整数类型。
    ///
    /// Floating point value.
    Int(i32),
    /// 布尔类型。
    ///
    /// Boolean value.
    Bool(bool),
    /// 浮点类型。
    ///
    /// Floating point value.
    Float(f32),
}

/// 节点的附加信息，包括 settings、options、依赖和兼容性等。
///
/// Additional node information (settings, options, requirements, compatibility).
/// 引文：
/// - [NodeInfo](https://github.com/conan-io/conan/blob/develop/conans/model/info.py)
#[derive(Debug, Deserialize)]
pub struct NodeInfo {
    /// 该节点的构建设置。
    ///
    /// Build settings for this node.
    pub settings: Settings,
    /// 该节点的选项值，key 为选项名。
    ///
    /// Option values for this node, key is option name.
    pub options: Option<HashMap<String, ValueVariant>>,
    /// 该节点的依赖项列表。
    ///
    /// Requirements for this node.
    pub requires: Option<Vec<String>>,
    /// 兼容性差异信息。
    ///
    /// Compatibility delta information.
    pub compatibility_delta: Option<CompatibilityDelta>,
}

/// 节点的兼容性差异（settings 差异），用于描述兼容性调整。
///
/// Compatibility delta for a node (settings differences).
/// 引文：
/// - [CompatibilityDelta](https://github.com/conan-io/conan/blob/develop/conans/model/info.py)
#[derive(Debug, Deserialize)]
pub struct CompatibilityDelta {
    /// 兼容性差异的 (setting, value) 键值对列表。
    ///
    /// List of (setting, value) pairs that differ for compatibility.
    pub settings: Vec<(String, String)>,
}

/// Conan 构建设置（如 os、arch、compiler 等），描述包的构建环境。
///
/// Conan build settings (e.g. os, arch, compiler, etc.), describes the build environment for the package.
/// 引文：
/// - [Settings](https://docs.conan.io/2/reference/conanfile/attributes.html#settings)
/// - [settings.yml](https://docs.conan.io/2/reference/config_files/settings.html#reference-config-files-settings-yml)
#[derive(Debug, Deserialize)]
pub struct Settings {
    /// 操作系统（如 "Windows"、"Linux"）。
    ///
    /// Operating system (e.g. "Windows", "Linux").
    pub os: String,
    /// 架构（如 "x86_64"、"armv8"）。
    ///
    /// Architecture (e.g. "x86_64", "armv8").
    pub arch: String,
    /// 编译器名称（如 "gcc"、"Visual Studio"）。
    ///
    /// Compiler name (e.g. "gcc", "Visual Studio").
    pub compiler: Option<String>,
    /// C++ 标准版本（如 "17"）。
    ///
    /// C++ standard version (e.g. "17").
    #[serde(rename = "compiler.cppstd")]
    pub compiler_cppstd: Option<String>,
    /// 编译器运行时（如 "MD"、"MT"，Windows 下常用）。
    ///
    /// Compiler runtime (e.g. "MD", "MT"), commonly used on Windows.
    #[serde(rename = "compiler.runtime")]
    pub compiler_runtime: Option<String>,
    /// 编译器运行时类型（如 "Debug"、"Release"）。
    ///
    /// Compiler runtime type (e.g. "Debug", "Release").
    #[serde(rename = "compiler.runtime_type")]
    pub compiler_runtime_type: Option<String>,
    /// 编译器版本（如 "10"、"14"）。
    ///
    /// Compiler version (e.g. "10", "14").
    #[serde(rename = "compiler.version")]
    pub compiler_version: Option<String>,
    /// 构建类型（如 "Release"、"Debug"）。
    ///
    /// Build type (e.g. "Release", "Debug").
    pub build_type: Option<String>,
}

/// C++ build information for a Conan package/component.
/// 引文：
/// - [CppInfo](https://docs.conan.io/2/reference/conanfile/attributes.html#cppinfo)
/// - [CppInfo Modal](https://docs.conan.io/2/reference/conanfile/methods/package_info.html#conan-conanfile-model-cppinfo)
/// - [CppInfo in Conan source](https://github.com/conan-io/conan/blob/develop/conans/model/pkg_metadata.py)
#[derive(Debug, Deserialize)]
pub struct CppInfo {
    /// 头文件目录列表（如 include/），用于编译器查找头文件。
    pub includedirs: Option<Vec<String>>,
    /// 源文件目录列表（如 src/），通常用于 IDE 或调试。
    pub srcdirs: Option<Vec<String>>,
    /// 库文件目录列表（如 lib/），用于链接器查找静态/动态库。
    pub libdirs: Option<Vec<String>>,
    /// 资源文件目录列表（如 res/），用于存放非代码资源。
    pub resdirs: Option<Vec<String>>,
    /// 可执行文件目录列表（如 bin/），用于存放可执行程序。
    pub bindirs: Option<Vec<String>>,
    /// 构建目录列表，通常用于生成的中间文件。
    pub builddirs: Option<Vec<String>>,
    /// macOS/iOS 的 framework 目录列表。
    pub frameworkdirs: Option<Vec<String>>,
    /// 依赖的系统库名称列表（如 pthread、dl）。
    pub system_libs: Option<Vec<String>>,
    /// 依赖的 macOS/iOS frameworks 名称列表。
    pub frameworks: Option<Vec<String>>,
    /// 依赖的库名称列表（如 foo、bar），不带前缀/后缀。
    pub libs: Option<Vec<String>>,
    /// 需要定义的宏（如 FOO_EXPORTS）。
    pub defines: Option<Vec<String>>,
    /// 需要传递给 C 编译器的参数（如 -Wall）。
    pub cflags: Option<Vec<String>>,
    /// 需要传递给 C++ 编译器的参数（如 -std=c++17）。
    pub cxxflags: Option<Vec<String>>,
    /// 需要传递给链接器的参数（用于生成共享库）。
    pub sharedlinkflags: Option<Vec<String>>,
    /// 需要传递给链接器的参数（用于生成可执行文件）。
    pub exelinkflags: Option<Vec<String>>,
    /// 需要链接的对象文件列表（如 foo.o）。
    pub objects: Option<Vec<String>>,
    /// 指定 sysroot 路径，影响头文件和库的查找根目录。
    pub sysroot: Option<String>,
    /// 依赖的其他 Conan 包名称列表。
    pub requires: Option<Vec<String>>,
    /// 附加属性，包含 CMake 相关配置等。
    pub properties: Option<CppInfoProperties>, // TODO: CppInfoProperties of HashMap<String, ValueVariant>
    /// 可执行文件路径（如 foo.exe），用于工具包。
    pub exe: Option<String>,
    /// 包类型（如 "static-library"、"shared-library"、"executable"）。
    #[serde(rename = "type")]
    pub type_: Option<String>,
    /// 该包的主目录路径。
    pub location: Option<String>,
    /// 链接时使用的库文件路径。
    pub link_location: Option<String>,
    /// 该包支持的编程语言列表（如 ["C", "C++"]）。
    pub languages: Option<Vec<String>>,
}

/// Properties for CMake and build system integration.
///
/// 引文：
/// - [CppInfoProperties](https://docs.conan.io/2/reference/conanfile/methods/package_info.html#conan-conanfile-model-cppinfo)
///
/// **Note**:
///
/// > set_property() allows to define some built-in and user general properties to be propagated with the cpp_info model for
/// > consumers. They might contain build-system specific information. Some built-in properties are `cmake_file_name`,
/// > `cmake_target_name`, `pkg_config_name`, that can define specific behavior for CMakeDeps or PkgConfigDeps generators.
/// > For more information about these, read the specific build system integration documentation.
#[derive(Debug, Deserialize)]
pub struct CppInfoProperties {
    /// CMake 查找模式（如 "config" 或 "module"）。
    pub cmake_find_mode: Option<String>,
    /// CMake 生成的包文件名（如 "Foo"）。
    pub cmake_file_name: Option<String>,
    /// CMake 生成的 target 名称（如 "Foo::Foo"）。
    pub cmake_target_name: Option<String>,
    /// 需要包含的 CMake 模块文件列表。
    pub cmake_build_modules: Option<Vec<String>>,
}

/// Dependency information for a node in the graph.
/// 引文：
/// - [Dependency](https://docs.conan.io/2/reference/conanfile/attributes.html#requirements)
/// - [Dependency in Conan source](https://github.com/conan-io/conan/blob/develop/conans/model/graph.py)
#[derive(Debug, Deserialize)]
pub struct Dependency {
    /// 依赖的 Conan 包引用（如 "foo/1.0@user/channel"）。
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
    /// 依赖的名称（如 "foo"）。
    pub require: Option<String>,
    /// 是否为运行时依赖。
    pub run: Option<bool>,
    /// 是否为库依赖。
    pub libs: Option<bool>,
    /// 是否跳过该依赖。
    pub skip: Option<bool>,
    /// 是否为测试依赖。
    pub test: Option<bool>,
    /// 是否强制依赖。
    pub force: Option<bool>,
    /// 是否为直接依赖。
    pub direct: Option<bool>,
    /// 是否为构建依赖。
    pub build: Option<bool>,
    /// 传递的头文件列表。
    pub transitive_headers: Option<bool>,
    // pub transitive_headers: Option<Vec<String>>,
    /// 传递的库文件列表。
    pub transitive_libs: Option<bool>,
    // pub transitive_libs: Option<Vec<String>>,
    /// 是否包含头文件。
    pub headers: Option<bool>,
    /// 包 ID 计算模式。
    pub package_id_mode: Option<String>,
    /// 该依赖是否可见。
    pub visible: Option<bool>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn test_parse_windows_hyperscan_json() -> anyhow::Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let path = std::path::Path::new(&manifest_dir).join("tests/windows-hyperscan.json");
        let data = fs::read_to_string(&path)?;
        let info: ConanRoot = serde_json::from_str(&data)?;

        assert!(info.graph.nodes.len() > 0);

        Ok(())
    }

    #[test]
    fn test_parse_windows_zlib_json() -> anyhow::Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let path = std::path::Path::new(&manifest_dir).join("tests/windows-zlib.json");
        let data = fs::read_to_string(&path)?;
        let info: ConanRoot = serde_json::from_str(&data)?;

        assert!(info.graph.nodes.len() > 0);
        Ok(())
    }

    #[test]
    fn test_parse_linux_hyperscan_json() -> anyhow::Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let path = std::path::Path::new(&manifest_dir).join("tests/linux-hyperscan.json");
        let data = fs::read_to_string(&path)?;
        let info: ConanRoot = serde_json::from_str(&data)?;

        assert!(info.graph.nodes.len() > 0);

        Ok(())
    }
}
