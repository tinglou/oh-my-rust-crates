//! OUI 数据文件解析与布局支持库
//!
//! 本 crate 分两部分：
//! - [`layout`]（始终可用，`no_std`）：OUI 查询表的列式内存布局类型与编码，
//!   由 `macaddr-ouidb` 的 bundled / owned 数据库共同使用。
//! - 解析与构建（`std` feature）：解析 `PREFIX CC ORG` 格式的 OUI 数据文件及文件头注释，
//!   供 `macaddr-ouidb` 的 `build.rs` 在构建时调用。只负责解析与校验，不涉及代码生成。

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod layout;

#[cfg(feature = "std")]
mod parse;

#[cfg(feature = "std")]
pub mod builder;

#[cfg(feature = "std")]
pub use parse::{
    OuiEntry, parse_oui_file, parse_oui_reader, parse_prefix_hex, parse_updated_reader,
};

#[cfg(feature = "std")]
pub use builder::{Oui28Entry, Oui36Entry, OuiDataBuilder, SubtableRef};
