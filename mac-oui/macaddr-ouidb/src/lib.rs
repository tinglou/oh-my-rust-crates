#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod macaddr;
mod macaddr8;
mod oui;
mod oui_data;

pub use macaddr::*;
pub use macaddr8::*;
pub use oui::*;
pub use oui_data::OUI_DB_BUNDLED;
