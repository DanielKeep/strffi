use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::cmp::Ordering;
use std::error::Error as StdError;
use std::fmt::{self, Debug};
use std::iter::FromIterator;
use std::mem;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeFull};
use libc::{c_char};
use alloc::{AllocError, Malloc};
use encoding::{MbUnit, MultiByte};
use sea::{SeStr, SeaString};
use structure::ZeroTerm;

macro_rules! nyi {
    () => (panic!("nyi"))
}

type ZMbStrInner = SeStr<ZeroTerm, MultiByte>;
type ZMbCStringInner = SeaString<ZeroTerm, MultiByte, Malloc>;

/**
Represents a borrowed C string.

Specifically, a zero-terminated string of units encoded in the current, thread-local C multibyte encoding, typically represented in foreign interfaces as `*const c_char` or `*mut c_char`.  It should be noted that this *is not* the same as ASCII, UTF-8, or the current Windows ANSI codepage.

You should *not* attempt to construct or use *values* of this type.  You should only ever use pointers to this type.  In future, this type may be redefined to be dynamically sized.

Pointers to `ZMbStr` can be obtained either by borrowing from a `ZMbCString`, by converting from a `SeStr<ZeroTerm, MultiByte>` pointer, or by converting from a raw FFI pointer type.

Note that this type *never* transfers ownership.  Passing a `ZMbStr` to a foreign interface expecting an *owned* string will likely result in a double-free error.  Converting an owned string from a foreign interface to a `ZMbStr` will result in a memory leak.

This type *may* be used in FFI signatures and types, but we nonetheless recommend not doing so, and explicitly using the `from_ptr` and `as_ptr` methods instead.

See also: `ZMbCString`.
*/
#[repr(C)]
pub struct ZMbStr(ZMbStrInner);

impl ZMbStr {
    /**
    Re-borrows a `ZMbStr` from a foreign string pointer.

    This method *does not* inspect the foreign string, or compute its length.

    If `ptr` is null, returns `None`.  Otherwise, it returns a valid pointer.

    # Safety

    If the foreign string pointed to by `ptr` is not zero-terminated, then the result of this method is invalid, and may result in a memory protection failure on use.

    It is impossible to know for how long the provided pointer will remain valid.  Care should be taken to ensure that the returned `ZMbStr` *does not* outlive the original foreign string.

    If you are uncertain as to the valid lifetime of `ptr`, you should *immediately* call `to_owned` on the result, and discard the intermediate result of `from_ptr`.
    */
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> Option<&'a Self> {
        SeStr::from_ptr(ptr).map(Into::into)
    }

    /**
    Mutably re-borrows a `ZMbStr` from a foreign string pointer.

    This method *does not* inspect the foreign string, or compute its length.

    If `ptr` is null, returns `None`.  Otherwise, it returns a valid pointer.

    # Safety

    If the foreign string pointed to by `ptr` is not zero-terminated, then the result of this method is invalid, and may result in a memory protection failure on use.

    It is impossible to know for how long the provided pointer will remain valid.  Care should be taken to ensure that the returned `ZMbStr` *does not* outlive the original foreign string.
    */
    pub unsafe fn from_ptr_mut<'a>(ptr: *mut c_char) -> Option<&'a mut Self> {
        SeStr::from_ptr_mut(ptr).map(Into::into)
    }

    /**
    Returns the units comprising this string as a contiguous slice.  This *does not* include the terminating zero.

    # Efficiency

    Note that this method will require a complete traversal of the underlying memory in order to compute the string's length.  You should avoid calling this method repeatedly.    
    */
    pub fn as_units(&self) -> &[MbUnit] {
        self.0.as_units()
    }

    /**
    Returns the units comprising this string as a contiguous slice.  This *includes* the terminating zero.

    # Efficiency

    Note that this method will require a complete traversal of the underlying memory in order to compute the string's length.  You should avoid calling this method repeatedly.    
    */
    pub fn as_units_with_term(&self) -> &[MbUnit] {
        self.0.as_units_with_term()
    }

    /**
    Returns the units comprising this string as a contiguous, mutable slice.  This *does not* include the terminating zero.

    # Safety

    This method is not memory-unsafe; here, `unsafe` is used as a check against questionable behaviour.

    Because this method excludes the terminating zero, it is not possible to accidentally "un-terminate" the string.  However, it *is* possible to introduce interior terminators into the string, altering its apparent length.  Any such truncation is permanent, and cannot be undone.
    */
    pub unsafe fn as_units_mut_unsafe(&mut self) -> &mut [MbUnit] {
        self.0.as_units_mut_unsafe()
    }

    /**
    Re-borrows this string as a foreign pointer.

    The returned pointer is valid for at least as long as the `ZMbStr` itself is.
    */
    pub fn as_ptr(&self) -> *const c_char {
        self.0.as_ptr()
    }

    /**
    Mutably re-borrows this string as a foreign pointer.

    The returned pointer is valid for at least as long as the `ZMbStr` itself is.
    */
    pub fn as_ptr_mut(&mut self) -> *mut c_char {
        self.0.as_ptr_mut()
    }

    /**
    Converts the contents of this string into a normal Rust string.

    # Failure

    This conversion will fail if the string contains any units which cannot be translated into Unicode.
    */
    pub fn into_string(&self) -> Result<String, Box<StdError>> {
        self.0.into_string()
    }
}

impl Debug for ZMbStr {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl Deref for ZMbStr {
    type Target = SeStr<ZeroTerm, MultiByte>;

    fn deref(&self) -> &Self::Target {
        self.into()
    }
}

impl DerefMut for ZMbStr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.into()
    }
}

impl<'a> From<&'a SeStr<ZeroTerm, MultiByte>> for &'a ZMbStr {
    fn from(v: &'a SeStr<ZeroTerm, MultiByte>) -> Self {
        unsafe { mem::transmute::<&ZMbStrInner, &ZMbStr>(v) }
    }
}

impl<'a> From<&'a mut SeStr<ZeroTerm, MultiByte>> for &'a mut ZMbStr {
    fn from(v: &'a mut SeStr<ZeroTerm, MultiByte>) -> Self {
        unsafe { mem::transmute::<&mut ZMbStrInner, &mut ZMbStr>(v) }
    }
}

impl<'a> From<&'a ZMbStr> for &'a SeStr<ZeroTerm, MultiByte> {
    fn from(v: &'a ZMbStr) -> Self {
        unsafe { mem::transmute::<&ZMbStr, &ZMbStrInner>(v) }
    }
}

impl<'a> From<&'a mut ZMbStr> for &'a mut SeStr<ZeroTerm, MultiByte> {
    fn from(v: &'a mut ZMbStr) -> Self {
        unsafe { mem::transmute::<&mut ZMbStr, &mut ZMbStrInner>(v) }
    }
}

impl ToOwned for ZMbStr {
    type Owned = ZMbCString;

    fn to_owned(&self) -> ZMbCString {
        self.0.to_owned_by::<Malloc>().expect("failed to allocate ZMbCString").into()
    }
}

/**
Represents an owned C string.

Specifically, a zero-terminated string of units encoded in the current, thread-local C multibyte encoding, typically represented in foreign interfaces as `*mut c_char`.  It should be noted that this *is not* the same as ASCII, UTF-8, or the current Windows ANSI codepage.

`ZMbCString`s can be constructed either from slices of units, by converting from a `SeaString<ZeroTerm, Multibyte, Malloc>`, by using `to_owned` on a `ZMbStr`, or by taking ownership from a raw FFI pointer type.

Note that this type *always* transfers ownership.  Passing a `ZMbCString` to a foreign interface expecting a *borrowed* string will result in a memory leak.  Taking ownership of a borrowed string from a foreign interface will likely result in double-free or heap errors.

`ZMbCString`s can be converted trivially into a `ZMbStr` pointer, via `AsRef`/`AsMut`, `Borrow`/`BorrowMut`, or dereferencing.  Although mutation is supported, zero termination does not permit *safe* mutation; see `ZMbStr` for available methods.

This type *may* be used in FFI signatures and types, but we nonetheless recommend not doing so, and explicitly using the `from_ptr` and `as_ptr` methods instead.

See also: `ZMbCString`.
*/
#[repr(C)]
pub struct ZMbCString(ZMbCStringInner);

impl ZMbCString {
    /**
    Construct a `ZMbCString` from a slice of units.

    # Failure

    This method will fail if allocating memory fails.

    Construction can also fail if the string contains zero units anywhere *other* than at the end.
    */
    // TODO: what about interior zeroes?
    pub fn new(units: &[MbUnit]) -> Result<Self, AllocError> {
        ZMbCStringInner::new(units).map(Into::into)
    }

    /**
    Construct a `ZMbCString` from a Rust string.

    # Failure

    This method will fail if allocating memory fails.

    Construction can also fail if the string contains zero units anywhere *other* than at the end.

    An error will also be returned if the contents of the input string cannot be transcoded to the C multi-byte encoding.
    */
    pub fn from_str<'a>(s: &'a str) -> Result<Self, Box<StdError>> {
        SeaString::from_str(s).map(Into::into)
    }

    /**
    Constructs a `ZMbCString` by taking ownership of a foreign string pointer.

    This method will not inspect the foreign string, or compute its length.

    If `ptr` is null, this method will return `None`; otherwise it will return a valid `ZMbCString`.

    # Safety

    If the `ptr` is not a valid pointer to a compatible foreign string, then the result of this method is invalid, and may result in a memory protection failure on use.

    This method must *not* be called more than once on the same pointer.
    */
    pub unsafe fn from_ptr(ptr: *mut c_char) -> Option<Self> {
        ZMbCStringInner::from_ptr(ptr).map(Into::into)
    }

    /**
    Relinquishes ownership of this string and returns a pointer.

    This pointer can be turned back into a `ZMbCString` by `from_ptr`, or sent to foreign code, which is then responsible for deallocating it.
    */
    pub fn into_ptr(self) -> *mut c_char {
        self.0.into_ptr()
    }
}

impl AsMut<ZMbStr> for ZMbCString {
    fn as_mut(&mut self) -> &mut ZMbStr {
        self
    }
}

impl AsRef<ZMbStr> for ZMbCString {
    fn as_ref(&self) -> &ZMbStr {
        self
    }
}

impl Borrow<ZMbStr> for ZMbCString {
    fn borrow(&self) -> &ZMbStr {
        self
    }
}

impl BorrowMut<ZMbStr> for ZMbCString {
    fn borrow_mut(&mut self) -> &mut ZMbStr {
        self
    }
}

impl Debug for ZMbCString {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl Default for ZMbCString {
    fn default() -> Self {
        ZMbCString::new(&[]).expect("could not allocate ZMbCString")
    }
}

impl Deref for ZMbCString {
    type Target = ZMbStr;

    fn deref(&self) -> &ZMbStr {
        self.0.deref().into()
    }
}

impl DerefMut for ZMbCString {
    fn deref_mut(&mut self) -> &mut ZMbStr {
        self.0.deref_mut().into()
    }
}

impl Eq for ZMbCString {}

impl From<SeaString<ZeroTerm, MultiByte, Malloc>> for ZMbCString {
    fn from(v: SeaString<ZeroTerm, MultiByte, Malloc>) -> Self {
        ZMbCString(v)
    }
}

impl From<ZMbCString> for SeaString<ZeroTerm, MultiByte, Malloc> {
    fn from(v: ZMbCString) -> Self {
        v.0
    }
}

impl FromIterator<MbUnit> for ZMbCString {
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item=MbUnit> {
        SeaString::from_iter(iter).into()
    }
}

impl Index<RangeFull> for ZMbCString {
    type Output = ZMbStr;

    fn index(&self, _index: RangeFull) -> &ZMbStr {
        self
    }
}

impl IndexMut<RangeFull> for ZMbCString {
    fn index_mut(&mut self, _index: RangeFull) -> &mut ZMbStr {
        self
    }
}

impl PartialEq<ZMbCString> for ZMbCString {
    fn eq(&self, other: &ZMbCString) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl PartialEq<ZMbStr> for ZMbCString {
    fn eq(&self, other: &ZMbStr) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl PartialEq<ZMbCString> for ZMbStr {
    fn eq(&self, other: &ZMbCString) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl PartialOrd<ZMbCString> for ZMbCString {
    fn partial_cmp(&self, other: &ZMbCString) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl PartialOrd<ZMbStr> for ZMbCString {
    fn partial_cmp(&self, other: &ZMbStr) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl PartialOrd<ZMbCString> for ZMbStr {
    fn partial_cmp(&self, other: &ZMbCString) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl Ord for ZMbCString {
    fn cmp(&self, other: &ZMbCString) -> Ordering {
        self.as_units().cmp(other.as_units())
    }
}
