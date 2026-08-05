#[allow(non_camel_case_types)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/hyperscan.rs"));
}

pub use bindings::*;
