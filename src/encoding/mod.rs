/*!
Encoding types and traits.
*/
pub mod conv;

use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use libc::{c_char, wchar_t};

/**
This trait abstracts over different encoding schemes for strings used in foreign code.

In practice, this will be implemented by a marker type (which are not intended to actually be instantiated anywhere), along with a concrete type that implements `Unit`, and likely at least one implementation of `TranscodeTo`.
*/
pub trait Encoding {
    /**
    The type that represents units in this encoding.

    This type should be unique, to prevent mixing up units from different encodings.  Often, this will be a newtype around the underlying storage type used for the encoding.

    The type chosen should *not* have any invariants on its values.  Remember that string contents can come from foreign languages that fail to encourage a stringent adherence to data validity.  One notable exception to this is the `CheckedUnicode` implementation which uses `char` for this type.  That encoding is a special case and is *specifically* not for use with foreign code.

    This type will be exposed to users as the contents of the corresponding string types.
    */
    type Unit: Unit;

    /**
    The type that represents units in this encoding in a foreign context.

    This should be chosen as the most common unit representation type for strings with this encoding in foreign interfaces.  This type does *not* need to be distinct.

    This type will *likely* be exposed to users via the high-level FFI conversion methods.
    */
    type FfiUnit;

    /**
    Returns a string which can be used to uniquely identify this encoding in debug output.

    This string should *preferably* be short, reasonably evocative, unique, and a single `Camelword`, although nothing will break if this is not done.

    For context, the debug representation of `SeStr` and `SeaString` involves concatenating the debug prefixes of the structure, encoding, and allocator together.
    */
    fn debug_prefix() -> &'static str;

    /**
    This must return a slice of units which begins with *at least* two consecutive zero units.

    This exists to allow for empty borrowed string instances to be cheaply constructed.  The requirement for at least *two* zeroes is to allow for double-terminated structures to be supported.
    */
    // TODO: Should this go into an unsafe trait?
    // TODO: Return a &[Self::Unit; 2] instead?
    fn static_zeroes() -> &'static [Self::Unit];
}

/**
Defines the interface for string units.

Implementations of this serve a similar purpose to `u8` for Rust strings: they are the smallest, indivisible unit of encoding for a string, and do not necessarily represent a single, complete character.
*/
pub trait Unit: Copy + PartialEq + Eq + PartialOrd + Ord + Hash + UnitDebug + 'static {
    /**
    Returns a zero unit.  This must be all zeroes, as suitable as a terminator for a zero-terminated string.

    To be clear, I mean the equivalent of `'\u{0}'`, *not* a literal `'0'`.
    */
    fn zero() -> Self;

    /**
    Determines if a given unit is equal to the zero unit.
    */
    fn is_zero(&self) -> bool;
}

/**
Formats a unit for debug output.

This is used on individual units in a string during debug formatting of the string as a whole.  As such, the output should be unambiguous, and *not* contain any enclosing quotes.

For encodings that are a superset of ASCII, printable ASCII units may be emitted directly.  Other units should output either a Unicode code point escape sequence (if the corresponding Unicode code point is known), or one or more raw binary escapes (*i.e.* `\xHH`).  Printable non-ASCII units should *not* be printed directly, as output encodings on the actual display terminal may mangle or replace such units.

An encoding may assume ASCII compatibility if such compatibility is reasonably likely, and not assuming such would lead to unreadable output on simple text.

Implementations should allow strings to be quickly transformed into a useful debug representation, and is *not* intended for actual user-facing display.  User-facing display is intended to be accomplished by transcoding the input string as a whole into a Rust string.
*/
pub trait UnitDebug {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result;
}

/**
Implementations of this trait define conversions from the implementing encoding to a given destination encoding.

In general, the intention is that all implementations of this trait should follow the form:

```ignore
impl<It> TranscodeTo<DstEnc> for UnitIter<SrcEnc, It>
where It: Iterator<Item=SrcEnc::Unit> {
    type Iter = SrcToDstIter<It>;
    type Error = SrcToDstError;

    fn transcode(self) -> Self::Iter {
        SrcToDstIter::new(self.into_iter())
    }
}
```

In particular, note the use of `UnitIter` as the implementing type.  This is because using iterator types directly has two drawbacks:

1. Doing so does not establish a direct link between the transcode implementation and the source encoding type.

2. Due to how coherence is checked, it does not allow more than one `TranscodeTo<DstEnc>` implementation to exist.

Using `UnitIter` solves both of these problems.

Implementations are *not* assumed to be reflexive (*i.e.* just because *A*→*B* exists does not imply *B*→*A* exists).  If a conversion *is* reflexive, two implementations must be written.

Implementations **must** support embedded terminators.  They should also be as lazy as possible.

In regards to failures, implementations should try to support "lossy" transcoding if possible.  This means that an implementation might emit an `Err` in place of an invalid or inconvertible unit or character, then resume emitting transcoded units afterward.  Each individual `Err` will likely be substituted with a replacement unit/character.  Implementations which support recovery can indicate this by implementing the `Recoverable` trait on the `Iter` type.

If an implementation cannot recover from transcoding failures, it *must* fuse itself and return `None` on all subsequent iterations after a single `Err`.
*/
pub trait TranscodeTo<Dst>: Sized where Dst: Encoding {
    /**
    The iterator type that represents an in-progress transcode.
    */
    type Iter: Iterator<Item=Result<Dst::Unit, Self::Error>>;

    /**
    The error type used to communicate transcoding failure.
    */
    type Error: ::std::error::Error + 'static;

    /**
    Begin transcoding from the `Self` encoding to the `Dst` encoding.
    */
    fn transcode(self) -> Self::Iter;
}

/**
A type used to tie encodings and structural iterators together.

This generally appears in constraints as `UnitIter<E, S::Iter>: TranscodeTo<F>`.  This means that there must be an implementation that can transcode strings with encoding `E` into encoding `F`, reading the string contents through the `S::Iter` iterator type.

# Motivation

This type *actually* exists to solve coherence issues with `TranscodeTo` implementations.

See the `TranscodeTo` trait for details.
*/
pub struct UnitIter<E, It>
where
    It: Iterator<Item=E::Unit>,
    E: Encoding,
{
    iter: It,
    _marker: PhantomData<E>,
}

impl<E, It> UnitIter<E, It>
where
    It: Iterator<Item=E::Unit>,
    E: Encoding,
{
    pub fn new(iter: It) -> Self {
        UnitIter {
            iter: iter,
            _marker: PhantomData,
        }
    }

    pub fn into_iter(self) -> It {
        self.iter
    }
}

/**
If implemented on an iterator, indicates that it can recover from transcoding errors.
*/
// TODO: add support to string types.
pub trait Recoverable {}

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

        impl Debug for $ty_name {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "'")?;
                UnitDebug::fmt(self, fmt)?;
                write!(fmt, "'")
            }
        }

        impl Ord for $ty_name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl PartialOrd for $ty_name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
    };
}

macro_rules! ascii_ext_unit_impl {
    ($ty_name:ident {format: $format:expr, unit_ty: $unit_ty:ty}) => {
        impl UnitDebug for $ty_name {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                if 0x20 <= self.0 && self.0 <= 0x7e {
                    Display::fmt(&(self.0 as u8 as char), fmt)
                } else {
                    write!(fmt, $format, self.0 as $unit_ty)
                }
            }
        }
    };
}

/**
Represents the current, thread-specific C runtime multi-byte encoding.

This depends on the current locale as controlled by the `setlocale` function.
*/
pub enum MultiByte {}

impl Encoding for MultiByte {
    type Unit = MbUnit;
    type FfiUnit = c_char;

    #[inline]
    fn debug_prefix() -> &'static str { "Mb" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [MbUnit] = &[MbUnit(0), MbUnit(0)];
        ZEROES
    }
}

/**
A string unit encoded in the current, thread-specific C runtime multi-byte encoding.
*/
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct MbUnit(pub c_char);

naive_unit_impl! { MbUnit }
ascii_ext_unit_impl! { MbUnit { format: "\\x{:02x}", unit_ty: u8 }}

/**
Represents the C runtime wide encoding.
*/
pub enum Wide {}

impl Encoding for Wide {
    type Unit = WUnit;
    type FfiUnit = wchar_t;

    #[inline]
    fn debug_prefix() -> &'static str { "W" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [WUnit] = &[WUnit(0), WUnit(0)];
        ZEROES
    }
}

/**
A string unit encoded in the C runtime wide encoding.
*/
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct WUnit(pub wchar_t);

naive_unit_impl! { WUnit }

impl UnitDebug for WUnit {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if 0x20 <= self.0 && self.0 <= 0x7e {
            Display::fmt(&(self.0 as u8 as char), fmt)
        } else {
            use ::util::Unsigned;
            let mut v = self.0.unsigned();
            for _ in 0..mem::size_of::<wchar_t>() {
                let b = (v & 0xff) as u8;
                write!(fmt, "\\x{:02x}", b)?;
                v >>= 8;
            }
            Ok(())
        }
    }
}

/**
Represents the UTF-8 encoding.

Note that this encoding is *not* assumed to be valid; strings in this encoding *may* contain invalid sequences, or decode to invalid code points.
*/
pub enum Utf8 {}

impl Encoding for Utf8 {
    type Unit = Utf8Unit;
    type FfiUnit = u8;

    #[inline]
    fn debug_prefix() -> &'static str { "Utf8" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [Utf8Unit] = &[Utf8Unit(0), Utf8Unit(0)];
        ZEROES
    }
}

/**
A string unit encoded in the UTF-8 encoding.
*/
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf8Unit(pub u8);

naive_unit_impl! { Utf8Unit }
ascii_ext_unit_impl! { Utf8Unit { format: "\\x{:02x}", unit_ty: u8 }}

/**
Represents the UTF-16 encoding.

Note that this encoding is *not* assumed to be valid; strings in this encoding *may* contain invalid sequences, or decode to invalid code points.
*/
pub enum Utf16 {}

impl Encoding for Utf16 {
    type Unit = Utf16Unit;
    type FfiUnit = u16;

    #[inline]
    fn debug_prefix() -> &'static str { "Utf16" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [Utf16Unit] = &[Utf16Unit(0), Utf16Unit(0)];
        ZEROES
    }
}

/**
A string unit encoded in the UTF-16 encoding.
*/
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf16Unit(pub u16);

naive_unit_impl! { Utf16Unit }
ascii_ext_unit_impl! { Utf16Unit { format: "\\u{:04x}", unit_ty: u16 }}

/**
Represents the UTF-32 encoding.

Note that this encoding is *not* assumed to be valid; strings in this encoding *may* contain invalid code points.
*/
pub enum Utf32 {}

impl Encoding for Utf32 {
    type Unit = Utf32Unit;
    type FfiUnit = u32;

    #[inline]
    fn debug_prefix() -> &'static str { "Utf32" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [Utf32Unit] = &[Utf32Unit(0), Utf32Unit(0)];
        ZEROES
    }
}

/**
A string unit encoded in the UTF-32 encoding.
*/
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Utf32Unit(pub u32);

naive_unit_impl! { Utf32Unit }
ascii_ext_unit_impl! { Utf32Unit { format: "\\U{:08x}", unit_ty: u32 }}

/**
Represents the UTF-32 encoding.

Note that this encoding is *required* to be valid; strings in this encoding *must not* contain invalid code points.
*/
pub enum CheckedUnicode {}

impl Encoding for CheckedUnicode {
    type Unit = char;
    type FfiUnit = char;

    #[inline]
    fn debug_prefix() -> &'static str { "U" }

    #[inline]
    fn static_zeroes() -> &'static [Self::Unit] {
        const ZEROES: &'static [char] = &['\u{0}', '\u{0}'];
        ZEROES
    }
}

impl Unit for char {
    fn zero() -> Self {
        '\u{0}'
    }

    fn is_zero(&self) -> bool {
        *self == '\u{0}'
    }
}

impl UnitDebug for char {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if ' ' <= *self && *self <= '~' {
            Display::fmt(self, fmt)
        } else {
            write!(fmt, "\\u{{{:x}}}", *self as u32)
        }
    }
}
