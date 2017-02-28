pub mod conv;

use libc::{c_char, wchar_t};

macro_rules! naive_unit_impl {
    ($ty_name:ident) => {
        impl Unit for $ty_name {
            #[inline]
            fn zero() -> Self {
                $ty_name(0)
            }

            #[inline]
            fn is_zero(&self) -> bool {
                self.0 == 0
            }
        }
    };
}

pub trait Encoding {
    type Unit: Unit;
    type FfiUnit;
}

pub trait Unit: Copy {
    fn zero() -> Self;
    fn is_zero(&self) -> bool;
}

pub trait TranscodeTo<Dst>: Sized {
    type Iter: Iterator<Item=Result<Dst, Self::Error>>;
    type Error: ::std::error::Error;

    fn transcode(self) -> Self::Iter;
}

pub enum MultiByte {}

impl Encoding for MultiByte {
    type Unit = MbUnit;
    type FfiUnit = c_char;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct MbUnit(c_char);

naive_unit_impl! { MbUnit }

pub enum Wide {}

impl Encoding for Wide {
    type Unit = WUnit;
    type FfiUnit = wchar_t;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct WUnit(wchar_t);

naive_unit_impl! { WUnit }

pub enum Utf8 {}

impl Encoding for Utf8 {
    type Unit = Utf8Unit;
    type FfiUnit = u8;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf8Unit(u8);

naive_unit_impl! { Utf8Unit }

pub enum Utf16 {}

impl Encoding for Utf16 {
    type Unit = Utf16Unit;
    type FfiUnit = u16;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf16Unit(u16);

naive_unit_impl! { Utf16Unit }

pub enum Utf32 {}

impl Encoding for Utf32 {
    type Unit = Utf32Unit;
    type FfiUnit = u32;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf32Unit(u32);

naive_unit_impl! { Utf32Unit }

pub enum CheckedUnicode {}

impl Encoding for CheckedUnicode {
    type Unit = char;
    type FfiUnit = char;
}

impl Unit for char {
    fn zero() -> Self {
        '\u{0}'
    }

    fn is_zero(&self) -> bool {
        *self == '\u{0}'
    }
}
