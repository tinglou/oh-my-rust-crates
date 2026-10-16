#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

mod macaddr;
mod oui;
mod oui_data;

pub use macaddr::*;
pub use oui::*;
pub use oui_data::OUI_DB;
