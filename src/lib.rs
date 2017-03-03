/*!
This crate defines types to help with interoperating with strings in other languages and environments.

For more details, see the [additional documentation](doc/index.html).

# Quick Reference

The following table describes some of the common string types, and when to use them.  Note that a single FFI type might map to multiple, mutually incompatible Rust types.  You should *always* consult the appropriate documentation to determine the exact properties you need to match.

This table does not remove the need to understand how this library represents strings.

| FFI Type | Documented properties | Rust Type |
| ---: | --- | --- |
| `*const c_char` | Pointer to character | `*const c_char` |
| … | Zero-terminated C string | `ZMbStr` |
| `*mut c_char` | Pointer to character | `*mut c_char` |
| … | *Unowned* zero-terminated C string | `ZMbStr` |
| … | *Owned* zero-terminated C string, using `malloc`/`free` | `ZMbCString` |
| `*const wchar_t` | Pointer to wide character | `*const wchar_t` |
| … | Zero-terminated wide C string | `ZWStr` |
| `*mut wchar_t` | Pointer to wide character | `*mut wchar_t` |
| … | *Unowned* zero-terminated wide C string | `ZWStr` |
| … | *Owned* zero-terminated wide C string, using `malloc`/`free` | `ZWCString` |
*/
#![cfg_attr(all(feature="nightly", feature="nightly-alloc"), feature(alloc, heap_api))]

extern crate libc;

#[cfg(all(feature="nightly", feature="nightly-alloc"))]
extern crate alloc as rust_alloc;

macro_rules! here { () => { &format!(concat!(file!(), ":{:?}"), line!()) } }

pub mod alloc;
#[doc(hidden)] pub mod doc;
pub mod encoding;
pub mod structure;
pub mod sea;

mod ffi;
mod util;
mod wrapper;

use alloc as a;
use encoding as e;
use structure as s;
use sea::{SeStr, SeaString};

pub type Error = Box<::std::error::Error>;

pub use wrapper::{ZMbStr, ZMbCString};

// pub type ZMbStr = SeStr<s::ZeroTerm, e::MultiByte>;
// pub type ZMbCString = SeaString<s::ZeroTerm, e::MultiByte, a::Malloc>;
// pub type ZMbRString = SeaString<s::ZeroTerm, e::MultiByte, a::Rust>;

pub type ZWStr = SeStr<s::ZeroTerm, e::Wide>;
pub type ZWCString = SeaString<s::ZeroTerm, e::Wide, a::Malloc>;
// pub type ZWRString = SeaString<s::ZeroTerm, e::Wide, a::Rust>;
