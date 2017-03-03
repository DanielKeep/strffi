/*!
Generalised FFI strings.
*/
use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::cmp::Ordering;
use std::convert::{AsRef, AsMut};
use std::error::Error as StdError;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeFull};

use alloc::{Allocator, Malloc};
use encoding::{Encoding, TranscodeTo, UnitDebug, CheckedUnicode};
use structure::{Structure, StructureAlloc, StructureDefault, MutationSafe, OwnershipTransfer, ZeroTerminated, Slice};
use util::{TrapErrExt, Utf8EncodeExt};

/**
Represents a borrowed foreign string.

You should *not* attempt to construct or use *values* of this type.  You should only ever use pointers to this type.  In future, this type may be redefined to be dynamically sized.

Pointers to `SeStr` can be obtained either by borrowing from a compatible `SeaString`, or by converting from a raw FFI pointer type.  Some concrete `SeStr` types may support other sources.

Note that this type *never* transfers ownership.  Passing a `SeStr` to a foreign interface expecting an *owned* string will likely result in a double-free or heap error.  Converting an owned string from a foreign interface to a `SeStr` will result in a memory leak.

This type *may* be used in FFI signatures and types, but we nonetheless recommend not doing so, and explicitly using the `from_ptr` and `as_ptr` methods instead.

# Parameters

`S` defines the structure of the string data.  *e.g.* `ZeroTerm` for zero-terminated strings, and `Slice` for Rust-style fat pointers.

`E` defines the encoding of the string data.  *e.g.* `MultiByte` for the current C runtime multibyte encoding, and `Wide` for C wide strings.
*/
pub struct SeStr<S, E> where S: Structure<E>, E: Encoding {
    data: S::RefTarget,
}

/**
This implementation is for strings that use native Rust slices as their structure.  In particular, it makes it possible to construct `SeStr` pointers without needing a new allocation.
*/
impl<E> SeStr<Slice, E> where E: Encoding {
    /**
    Creates a `SeStr<Slice, E>` pointer from a slice.
    */
    pub fn new(units: &[E::Unit]) -> &Self {
        unsafe {
            mem::transmute_copy::<&[E::Unit], &Self>(&units)
        }
    }

    /**
    Creates a mutable `SeStr<Slice, E>` pointer from a slice.
    */
    pub fn new_mut(units: &mut [E::Unit]) -> &mut Self {
        unsafe {
            mem::transmute_copy::<&mut [E::Unit], &mut Self>(&units)
        }
    }
}

/**
General implementation.
*/
impl<S, E> SeStr<S, E> where S: Structure<E>, E: Encoding {
    /**
    Re-borrows a `SeStr` from a foreign string pointer.

    This method will, ideally, not inspect the foreign string, or compute its length.

    If `ptr` is null, the result is dependent on the string's structure.  If null is not a valid string pointer value, this method will return `None`; otherwise it will return a valid `SeStr` pointer.

    # Safety

    If the `ptr` is not a valid pointer to a structurally compatible foreign string, then the result of this method is invalid, and may result in a memory protection failure on use.

    It is impossible to know for how long the provided pointer will remain valid.  Care should be taken to ensure that the returned `SeStr` *does not* outlive the original foreign string.

    If you are uncertain as to the valid lifetime of `ptr`, you should *immediately* call `to_owned` on the result, and discard the intermediate result of `from_ptr`.
    */
    pub unsafe fn from_ptr<'a>(ptr: S::FfiPtr) -> Option<&'a Self> {
        mem::transmute::<Option<&S::RefTarget>, _>(S::borrow_from_ffi_ptr(ptr))
    }

    /**
    Mutably re-borrows a `SeStr` from a foreign string pointer.

    This method will, ideally, not inspect the foreign string, or compute its length.

    If `ptr` is null, the result is dependent on the string's structure.  If null is not a valid string pointer value, this method will return `None`; otherwise it will return a valid `SeStr` pointer.

    # Safety

    If the `ptr` is not a valid pointer to a structurally compatible foreign string, then the result of this method is invalid, and may result in a memory protection failure on use.

    It is impossible to know for how long the provided pointer will remain valid.  Care should be taken to ensure that the returned `SeStr` *does not* outlive the original foreign string.

    If you are uncertain as to the valid lifetime of `ptr`, you should *immediately* call `to_owned` on the result, and discard the intermediate result of `from_ptr`.
    */
    pub unsafe fn from_ptr_mut<'a>(ptr: S::FfiMutPtr) -> Option<&'a mut Self> {
        mem::transmute::<Option<&mut S::RefTarget>, _>(S::borrow_from_ffi_ptr_mut(ptr))
    }

    /**
    Returns the units comprising the content of this string as a contiguous slice.  This *does not* include any structural data (including terminating units).

    # Efficiency

    For structures where the length of the string is not stored directly, this may require a complete traversal of the underlying memory.  You should avoid calling this method repeatedly.

    This method is guaranteed to be *O*(1) if `S` implements the `KnownLength` trait.
    */
    pub fn as_units(&self) -> &[E::Unit] {
        S::slice_units(&self.data)
    }

    /**
    Returns the units comprising the content of this string as a contiguous slice.  This *does not* include any structural data (including terminating units).

    # Efficiency

    For structures where the length of the string is not stored directly, this may require a complete traversal of the underlying memory.  You should avoid calling this method repeatedly.

    This method is guaranteed to be *O*(1) if `S` implements the `KnownLength` trait.

    # Safety

    This method is not memory-unsafe; here, `unsafe` is used as a check against questionable behaviour.

    Because this method excludes structural and terminating elements, it is not possible to accidentally corrupt the string.  However, it *is* possible to introduce interior terminators into the string, altering its apparent length with some representations.  Any such modification is permanent, and cannot be undone.

    See also: `as_units_mut`.
    */
    pub unsafe fn as_units_mut_unsafe(&mut self) -> &mut [E::Unit] {
        S::slice_units_mut(&mut self.data)
    }

    /**
    Re-borrows this string as a `SeStr<Slice, E>`.  This can be used to normalise string representations, or to "pre-compute" the length of a foreign string before further processing.
    */
    pub fn as_slice(&self) -> &SeStr<Slice, E> {
        SeStr::new(self.as_units())
    }

    /**
    Mutably re-borrows this string as a `SeStr<Slice, E>`.  This can be used to normalise string representations, or to "pre-compute" the length of a foreign string before further processing.

    # Safety

    This method is not memory-unsafe; here, `unsafe` is used as a check against questionable behaviour.

    Because this method can be used to turn a "not safe to mutate" string into a sliced string (which *are* safe to mutate), this method is unsafe by default.

    See also: `as_slice_mut`.
    */
    pub unsafe fn as_slice_mut_unsafe(&mut self) -> &mut SeStr<Slice, E> {
        SeStr::new_mut(self.as_units_mut_unsafe())
    }

    /**
    Re-borrows this string as a foreign pointer.

    The returned pointer is valid for at least as long as the `SeStr` itself is.
    */
    pub fn as_ptr(&self) -> S::FfiPtr {
        S::as_ffi_ptr(&self.data)
    }

    /**
    Mutably re-borrows this string as a foreign pointer.

    The returned pointer is valid for at least as long as the `SeStr` itself is.
    */
    pub fn as_ptr_mut(&mut self) -> S::FfiMutPtr {
        S::as_ffi_ptr_mut(&mut self.data)
    }

    /**
    Creates an owned string with the contents of this string, managed by the given allocator.

    # Failure

    This method can fail if the allocator is unable to allocate sufficient memory.
    */
    pub fn to_owned_by<A>(&self) -> Result<SeaString<S, E, A>, A::AllocError>
    where
        S: StructureAlloc<E, A>,
        A: Allocator,
    {
        SeaString::new(self.as_units())
    }

    /**
    Converts the contents of this string into a normal Rust string.

    # Failure

    This conversion will fail if the string contains any units which cannot be translated into Unicode.
    */
    pub fn into_string(&self) -> Result<String, Box<StdError>>
    where
        for<'a> &'a [E::Unit]: TranscodeTo<char>,
    {
        let mut err = Ok(());
        let units: Vec<_> = self
            .transcode_to_iter::<CheckedUnicode>()
            .trap_err(&mut err)
            .encode_utf8()
            .collect();
        let () = err?;
        let s = unsafe { String::from_utf8_unchecked(units) };
        Ok(s)
    }

    /**
    Transcodes the contents of this string into a different encoding.

    Note that this can also be used to copy the string contents into a string with a different structure.

    # Failure

    This conversion will fail if the string contains any units which cannot be translated into the target encoding, or if allocation fails.
    */
    pub fn transcode_to<U, F, A>(&self) -> Result<SeaString<U, F, A>, Box<StdError>>
    where
        U: Structure<F> + StructureAlloc<F, A>,
        F: Encoding,
        A: Allocator,
        for <'a> &'a [E::Unit]: TranscodeTo<F::Unit>,
    {
        let units: Result<Vec<_>, _> = self.transcode_to_iter::<F>().collect();
        let units = units?;
        Ok(SeaString::new(&units[..])?)
    }

    /**
    Transcodes the contents of this string into a different encoding.

    The transcoded string contents are returned as an iterator.

    # Failure

    This conversion will fail if the string contains any units which cannot be translated into the target encoding.
    */
    pub fn transcode_to_iter<'a, F>(&'a self) -> <&'a [E::Unit] as TranscodeTo<F::Unit>>::Iter
    where
        F: Encoding,
        &'a [E::Unit]: TranscodeTo<F::Unit>,
    {
        self.as_units().transcode()
    }

}

/**
This implementation only applies to string structures which are safe to mutate without the risk of truncation or corruption.
*/
impl<S, E> SeStr<S, E> where S: Structure<E> + MutationSafe, E: Encoding {
    /**
    Returns the units comprising the content of this string as a contiguous slice.  This *does not* include any structural data (including terminating units).

    # Efficiency

    For structures where the length of the string is not stored directly, this may require a complete traversal of the underlying memory.  You should avoid calling this method repeatedly.

    This method is guaranteed to be *O*(1) if `S` implements the `KnownLength` trait.
    */
    pub fn as_units_mut(&mut self) -> &mut [E::Unit] {
        unsafe { self.as_units_mut_unsafe() }
    }

    /**
    Mutably re-borrows this string as a `SeStr<Slice, E>`.  This can be used to normalise string representations, or to "pre-compute" the length of a foreign string before further processing.
    */
    pub fn as_slice_mut(&mut self) -> &mut SeStr<Slice, E> {
        unsafe { self.as_slice_mut_unsafe() }
    }
}

/**
This implementation only applies to string structures that end with a zero terminator.
*/
impl<S, E> SeStr<S, E> where S: ZeroTerminated<E>, E: Encoding {
    pub fn as_units_with_term(&self) -> &[E::Unit] {
        S::slice_units_with_term(&self.data)
    }
}

impl<S, E> AsMut<Self> for SeStr<S, E> where S: Structure<E>, E: Encoding {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<S, E> AsRef<Self> for SeStr<S, E> where S: Structure<E>, E: Encoding {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<S, E> Debug for SeStr<S, E> where S: Structure<E>, E: Encoding {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}{}\"", S::debug_prefix(), E::debug_prefix())?;
        for unit in self.as_units() {
            UnitDebug::fmt(unit, fmt)?;
        }
        write!(fmt, "\"")
    }
}

impl<'a, S, E> Default for &'a SeStr<S, E> where S: Structure<E> + StructureDefault<E>, E: Encoding {
    fn default() -> Self {
        unsafe { mem::transmute::<&S::RefTarget, &SeStr<_, _>>(S::default()) }
    }
}

impl<S, E> Eq for SeStr<S, E> where S: Structure<E>, E: Encoding {}

impl<S, E> Hash for SeStr<S, E> where S: Structure<E>, E: Encoding {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        Hash::hash_slice(self.as_units(), state)
    }
}

impl<S, E> Ord for SeStr<S, E>
where
    S: Structure<E>,
    E: Encoding,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_units().cmp(other.as_units())
    }
}

impl<S, E, T> PartialOrd<SeStr<T, E>> for SeStr<S, E>
where
    S: Structure<E>,
    E: Encoding,
    T: Structure<E>,
{
    fn partial_cmp(&self, other: &SeStr<T, E>) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl<S, E, T> PartialEq<SeStr<T, E>> for SeStr<S, E>
where
    S: Structure<E>,
    E: Encoding,
    T: Structure<E>,
{
    fn eq(&self, other: &SeStr<T, E>) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl<S, E> ToOwned for SeStr<S, E>
where
    S: Structure<E> + StructureAlloc<E, Malloc>,
    E: Encoding,
{
    type Owned = SeaString<S, E, Malloc>;

    fn to_owned(&self) -> SeaString<S, E, Malloc> {
        self.to_owned_by().expect("could not allocate SeaString")
    }
}

/**
Represents an owned foreign string.

`SeaString`s can be constructed either from slices of units, by transcoding a `SeStr`, by using `to_owned_as` on a `SeStr`, or by taking ownership from a raw FFI pointer type.

Note that this type *always* transfers ownership.  Passing a `SeaString` to a foreign interface expecting a *borrowed* string will result in a memory leak.  Taking ownership of a borrowed string from a foreign interface will likely result in double-free or heap errors.

`SeaString`s can be converted trivially into a corresponding `SeStr` type, via `AsRef`/`AsMut`, `Borrow`/`BorrowMut`, or dereferencing.  Although mutation is supported, not all structures permit *safe* mutation; see `SeStr` for available methods.

This type *may* be used in FFI signatures and types, but we nonetheless recommend not doing so, and explicitly using the `from_ptr` and `into_ptr` methods instead.

# Parameters

`S` defines the structure of the string data.  *e.g.* `ZeroTerm` for zero-terminated strings, and `Slice` for Rust-style fat pointers.

`E` defines the encoding of the string data.  *e.g.* `MultiByte` for the current C runtime multibyte encoding, and `Wide` for C wide strings.

`A` defines the allocator which manages the string data.  *e.g.* `Malloc` for the C runtime heap allocator, and `Rust` for the Rust heap allocator.
*/
pub struct SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    owned: S::Owned,
    _marker: PhantomData<A>,
}

/*impl<S, E, A> SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
}*/

/**
General methods.
*/
impl<S, E, A> SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    /**
    Construct a `SeaString` from a slice of units.

    # Failure

    This method will fail if allocating memory fails.

    Construction can also fail if the string contents provided are incompatible with the structure.  For example, it is invalid to construct a zero-terminated string with zero units in anywhere *other* than at the end.
    */
    // TODO: what about interior zeroes?
    pub fn new(units: &[E::Unit]) -> Result<Self, A::AllocError> {
        Ok(SeaString {
            owned: S::alloc_owned(units)?,
            _marker: PhantomData,
        })
    }
}

/**
Methods for structures that allow for transfer of ownership.
*/
impl<S, E, A> SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A> + OwnershipTransfer<E>,
    E: Encoding,
    A: Allocator,
{
    /**
    Constructs a `SeaString` by taking ownership of a foreign string pointer.

    This method will, ideally, not inspect the foreign string, or compute its length.

    If `ptr` is null, the result is dependent on the string's structure.  If null is not a valid string pointer value, this method will return `None`; otherwise it will return a valid `SeaString`.

    # Safety

    If the `ptr` is not a valid pointer to a structurally compatible foreign string, then the result of this method is invalid, and may result in a memory protection failure on use.

    This method must *not* be called more than once on the same pointer.  The only hypothetical exception would be strings which use shared ownership.
    */
    pub unsafe fn from_ptr(ptr: S::OwnedFfiPtr) -> Option<Self> {
        Some(SeaString {
            owned: match S::owned_from_ffi_ptr(ptr) {
                Some(owned) => owned,
                None => return None,
            },
            _marker: PhantomData,
        })
    }

    /**
    Relinquishes ownership of this string and returns a pointer.

    This pointer can be turned back into a `SeaString` by `from_ptr`, or sent to foreign code, which is then responsible for deallocating it.
    */
    pub fn into_ptr(mut self) -> S::OwnedFfiPtr {
        unsafe {
            let ptr = S::into_ffi_ptr(&mut self.owned);
            mem::forget(self);
            ptr
        }
    }
}

impl<S, E, A> AsMut<SeStr<S, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn as_mut(&mut self) -> &mut SeStr<S, E> {
        unsafe {
            mem::transmute::<&mut S::RefTarget, _>(S::borrow_from_owned_mut(&mut self.owned))
        }
    }
}

impl<S, E, A> AsRef<SeStr<S, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn as_ref(&self) -> &SeStr<S, E> {
        unsafe {
            mem::transmute::<&S::RefTarget, _>(S::borrow_from_owned(&self.owned))
        }
    }
}

impl<S, E, A> Borrow<SeStr<S, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn borrow(&self) -> &SeStr<S, E> {
        self
    }
}

impl<S, E, A> BorrowMut<SeStr<S, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn borrow_mut(&mut self) -> &mut SeStr<S, E> {
        self
    }
}

impl<S, E, A> Clone for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn clone(&self) -> Self {
        SeaString::new(self.as_units()).expect("could not allocate SeaString")
    }
}

impl<S, E, A> Debug for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}{}{}\"", S::debug_prefix(), E::debug_prefix(), A::debug_prefix())?;
        for unit in self.as_units() {
            UnitDebug::fmt(unit, fmt)?;
        }
        write!(fmt, "\"")
    }
}

impl<S, E, A> Default for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A> + StructureDefault<E>,
    E: Encoding,
    A: Allocator,
{
    fn default() -> Self {
        <&SeStr<S, E>>::default().to_owned_by::<A>().expect("could not allocate SeaString")
    }
}

impl<S, E, A> Deref for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    type Target = SeStr<S, E>;

    fn deref(&self) -> &SeStr<S, E> {
        unsafe {
            mem::transmute::<&S::RefTarget, _>(S::borrow_from_owned(&self.owned))
        }
    }
}

impl<S, E, A> DerefMut for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn deref_mut(&mut self) -> &mut SeStr<S, E> {
        unsafe {
            mem::transmute::<&mut S::RefTarget, _>(S::borrow_from_owned_mut(&mut self.owned))
        }
    }
}

impl<S, E, A> Drop for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn drop(&mut self) {
        S::free_owned(&mut self.owned);
    }
}

// impl<'a, S, E, A> From<&'a [E::Unit]> for SeaString<S, E, A>
// where
//     S: Structure<E> + StructureAlloc<E, A>,
//     E: Encoding,
//     A: Allocator,
// {
//     fn from(value: &'a [E::Unit]) -> Self {
//         let owned = ;
//         SeaString {
//             owned: S::alloc_owned::<A>(value)?,
//             _marker: PhantomData,
//         }
//     }
// }

impl<S, E, A> Eq for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{}

impl<S, E, A> FromIterator<E::Unit> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item=E::Unit> {
        let units: Vec<_> = iter.into_iter().collect();
        SeaString::new(&units[..]).expect("could not allocate SeaString")
    }
}

impl<S, E, A> Index<RangeFull> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    type Output = SeStr<S, E>;

    fn index(&self, _index: RangeFull) -> &SeStr<S, E> {
        self
    }
}

impl<S, E, A> IndexMut<RangeFull> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn index_mut(&mut self, _index: RangeFull) -> &mut SeStr<S, E> {
        self
    }
}

impl<S, E, A, T, B> PartialEq<SeaString<T, E, B>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
    T: Structure<E> + StructureAlloc<E, B>,
    B: Allocator,
{
    fn eq(&self, other: &SeaString<T, E, B>) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl<S, E, A, T> PartialEq<SeStr<T, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
    T: Structure<E>,
{
    fn eq(&self, other: &SeStr<T, E>) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl<S, E, T, B> PartialEq<SeaString<T, E, B>> for SeStr<S, E>
where
    S: Structure<E>,
    E: Encoding,
    T: Structure<E> + StructureAlloc<E, B>,
    B: Allocator,
{
    fn eq(&self, other: &SeaString<T, E, B>) -> bool {
        self.as_units().eq(other.as_units())
    }
}

impl<S, E, A, T, B> PartialOrd<SeaString<T, E, B>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
    T: Structure<E> + StructureAlloc<E, B>,
    B: Allocator,
{
    fn partial_cmp(&self, other: &SeaString<T, E, B>) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl<S, E, A, T> PartialOrd<SeStr<T, E>> for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
    T: Structure<E>,
{
    fn partial_cmp(&self, other: &SeStr<T, E>) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl<S, E, T, B> PartialOrd<SeaString<T, E, B>> for SeStr<S, E>
where
    S: Structure<E>,
    E: Encoding,
    T: Structure<E> + StructureAlloc<E, B>,
    B: Allocator,
{
    fn partial_cmp(&self, other: &SeaString<T, E, B>) -> Option<Ordering> {
        self.as_units().partial_cmp(other.as_units())
    }
}

impl<S, E, A> Ord for SeaString<S, E, A>
where
    S: Structure<E> + StructureAlloc<E, A>,
    E: Encoding,
    A: Allocator,
{
    fn cmp(&self, other: &SeaString<S, E, A>) -> Ordering {
        self.as_units().cmp(other.as_units())
    }
}
