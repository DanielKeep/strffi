#![cfg_attr(all(feature="nightly", feature="nightly-alloc"), feature(alloc, heap_api))]

extern crate libc;

#[cfg(all(feature="nightly", feature="nightly-alloc"))]
extern crate alloc as rust_alloc;

macro_rules! here {
    () => {
        &format!(concat!(file!(), ":{:?}"), line!())
    };
}

pub mod alloc;
pub mod encoding;
pub mod structure;
pub mod sea;
pub mod util;

mod ffi;

use alloc as a;
use encoding as e;
use structure as s;
use sea::{SeStr, SeaString};

pub type Error = Box<::std::error::Error>;

pub type ZMbStr = SeStr<s::ZeroTerm, e::MultiByte>;
pub type ZMbCString = SeaString<s::ZeroTerm, e::MultiByte, a::Malloc>;
pub type ZMbRString = SeaString<s::ZeroTerm, e::MultiByte, a::Rust>;

pub type ZWStr = SeStr<s::ZeroTerm, e::Wide>;
pub type ZWCString = SeaString<s::ZeroTerm, e::Wide, a::Malloc>;
pub type ZWRString = SeaString<s::ZeroTerm, e::Wide, a::Rust>;
