/*!
Structure types and traits.
*/
use std::mem;
use std::ptr;
use std::slice;
use alloc::{Allocator, AllocatorError};
use encoding::{Encoding, Unit};

/**
This trait is used to abstract over different kinds of string structures used in foreign code.

These structures are responsible for controlling how a string is laid out in memory, how its length is determined, and how handles to this memory are represented.

In practice, this will be implemented by a marker type (which are not intended to actually be instantiated anywhere), likely along with at least one implementation of `StructureAlloc`, and possibly implementations of the other traits in this module.
*/
pub trait Structure<E>: Sized where E: Encoding {
    /**
    Used to represent an owned handle to a string with this structure.  It serves a purpose similar to `String` for Rust strings.

    This type is never directly exposed to users, and is treated as a "black box" by `SeaString`.

    The type chosen *must not* have a non-trivial `Drop` implementation.  Deallocation is explicitly managed by `SeaString` through the `StructureAlloc` trait.
    */
    // TODO: move to StructureAlloc?
    type Owned;

    /**
    The *target* of borrowed pointers.  This serves a similar purpose to `str` for Rust strings.

    This type is never directly exposed to users, and is treated as a "black box" by `SeStr`.  In particular, although `SeStr` stores instances of this type by value, it *always* operates on them from behind some kind of pointer.

    If possible, a dynamically sized type should be used to make using the corresponding `SeStr` type by-value impossible.
    */
    type RefTarget: ?Sized;

    /**
    The "foreign" immutably borrowed pointer type.

    This type is exposed to users, and should match the most common FFI representation for strings with this structure.  This type is often defined in terms of `E::FfiUnit`.

    This type should be binary-compatible with `&RefTarget`.  The distinction exists for the sake of the public interface having "expected" types.
    */
    type FfiPtr;

    /**
    The foreign mutably borrowed pointer type.

    This type is exposed to users, and should match the most common FFI representation for mutable strings with this structure.  This type is often defined in terms of `E::FfiUnit`.

    This type should be binary-compatible with `&mut RefTarget`.  The distinction exists for the sake of the public interface having "expected" types.
    */
    type FfiMutPtr;

    /**
    Returns a string which can be used to uniquely identify this structure in debug output.

    This string should *preferably* be short, reasonably evocative, unique, and a single `Camelword`, although nothing will break if this is not done.

    For context, the debug representation of `SeStr` and `SeaString` involves concatenating the debug prefixes of the structure, encoding, and allocator together.
    */
    fn debug_prefix() -> &'static str;

    /**
    Constructs an immutably borrowed pointer to a string from the foreign pointer type.

    This should only return `None` if the provided foreign pointer is *invalid*.  In cases where the null pointer is a valid string (usually semantically equivalent to the empty string), it should return `Some` and *a valid pointer*.

    Implementations *may* perform basic sanity-checks within this method, but should avoid any non-*O*(1) work.  It is the caller's responsibility to ensure any non-null pointer provided is valid.

    # Motivation

    Why do we return an optional borrow rather than a pointer?  Because a null `*const [T]` isn't something Rust really wants to have as a thing, mostly because it raises the question of what the length is supposed to be.  Maybe "0" is the obvious answer.  What's *less* obvious is what the second word for `*const Trait` is supposed to be.

    So, rather than deal with that, or potentially invoke nasal demons, we just do something Rust *is* OK with.
    */
    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget>;

    /**
    The mutable sibling of `borrow_from_ffi_ptr`.  See that method for details.
    */
    unsafe fn borrow_from_ffi_ptr_mut<'a>(ptr: Self::FfiMutPtr) -> Option<&'a mut Self::RefTarget>;

    /**
    Given a valid immutably borrowed pointer, returns a slice over the contents of the string.

    If this method can be implemented in *O*(1), the `KnownLength` trait should *also* be implemented.
    */
    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit];

    /**
    The mutable sibling of `slice_units`.  See that method for details.
    */
    fn slice_units_mut(ptr: &mut Self::RefTarget) -> &mut [E::Unit];

    /**
    Given a pointer to an owned string, derives an immutably borrowed pointer.

    This does the moral equivalent of converting `&String` to `&str`.
    */
    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget;

    /**
    The mutable sibling of `borrow_from_owned`.  See that method for details.
    */
    fn borrow_from_owned_mut<'a>(owned: &mut Self::Owned) -> &mut Self::RefTarget;

    /**
    Given a valid immutably borrowed pointer, returns the corresponding foreign pointer.
    */
    fn as_ffi_ptr(ptr: &Self::RefTarget) -> Self::FfiPtr;

    /**
    The mutable sibling of `as_ffi_ptr`.  See that method for details.
    */
    fn as_ffi_ptr_mut(ptr: &mut Self::RefTarget) -> Self::FfiMutPtr;
}

/**
Specifies the interface used to allocate and deallocate strings.

The reason this trait exists in this form is to allow for cases where the structure of a string is inextricably tied to how it allocates its memory.  The canonical example of this is Windows' `BSTR`, which is required to *always* use a specific allocator, which itself is *only* capable of allocating `BSTR`s.

Most structures should use an implementation like the following:

```ignore
impl<E, A> StructureAlloc<E, A> for StructureMarkerType
where
    E: Encoding,
    A: Allocator<Pointer=*mut ()>,
{
    // ...
}
```

Specifically, note the use of the `Pointer=*mut ()` requirement.  Allocators which do not impose any particular limitation all produce `*mut ()` pointers.  Allocators which *do* impose limitations will produce a distinct type.
*/
pub trait StructureAlloc<E, A>: Structure<E> where E: Encoding, A: Allocator {
    /**
    Allocate a string with the given contents, and return an owned pointer.

    # Failure

    May fail if any of the underlying allocations fail.
    */
    // TODO: what about failing on invalid contents?
    fn alloc_owned(units: &[E::Unit]) -> Result<Self::Owned, A::AllocError>;

    /**
    Deallocate a string.
    */
    fn free_owned(ptr: &mut Self::Owned);
}

/**
Implemented by structures which define a valid "default" string.

This will almost always be some kind of empty string.
*/
pub trait StructureDefault<E>: Structure<E> where E: Encoding {
    /**
    Construct a default (likely empty) borrowed string.
    */
    fn default<'a>() -> &'a Self::RefTarget;
}

/**
This trait should be implemented for structures where computing the length is an *O*(1) operation.
*/
pub trait KnownLength {}

/**
This trait must *only* be implemented for structures where mutating the string's contents *cannot* change any other properties of the string.

In particular, this exists to gate mutable access to string types that use embedded terminators.
*/
pub unsafe trait MutationSafe {}

/**
This trait must *only* be implemented for structures where transferring ownership to and from foreign code is safe.
*/
// TODO: Does this *actually* need to be a separate, unsafe trait?  Maybe `Structure` itself should be unsafe.
pub unsafe trait OwnershipTransfer<E>: Structure<E> where E: Encoding {
    type OwnedFfiPtr;

    unsafe fn owned_from_ffi_ptr(ptr: Self::OwnedFfiPtr) -> Option<Self::Owned>;
    unsafe fn into_ffi_ptr(ptr: &mut Self::Owned) -> Self::OwnedFfiPtr;
}

/**
Implemented for structures which have an inline zero terminator.
*/
// TODO: what about double zero terminators?
pub trait ZeroTerminated<E>: Structure<E> where E: Encoding {
    /**
    Returns a slice of the string's contents, *including* the zero terminator.
    */
    fn slice_units_with_term(ptr: &Self::RefTarget) -> &[E::Unit];
}

/**
Strings represented by a pointer to the first unit, with a terminating zero unit.

This is the structure used by various forms of "C" string.  This should *not* be used for strings which feature a zero terminator, but also allow embedded zeroes by some other means.
*/
pub enum ZeroTerm {}

impl<E> Structure<E> for ZeroTerm where E: Encoding {
    type Owned = *mut ();
    type RefTarget = E::Unit;

    type FfiPtr = *const E::FfiUnit;
    type FfiMutPtr = *mut E::FfiUnit;

    fn debug_prefix() -> &'static str { "Z" }

    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
        if ptr.is_null () {
            None
        } else {
            Some(mem::transmute::<Self::FfiPtr, &Self::RefTarget>(ptr))
        }
    }

    unsafe fn borrow_from_ffi_ptr_mut<'a>(ptr: Self::FfiMutPtr) -> Option<&'a mut Self::RefTarget> {
        if ptr.is_null () {
            None
        } else {
            Some(mem::transmute::<Self::FfiPtr, &mut Self::RefTarget>(ptr))
        }
    }

    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit] {
        unsafe {
            let mut len = 0;
            let mut cur = ptr as *const E::Unit;

            while !(*cur).is_zero() {
                len += 1;
                cur = cur.offset(1);
            }

            ::std::slice::from_raw_parts(ptr as *const E::Unit, len)
        }
    }

    fn slice_units_mut(ptr: &mut Self::RefTarget) -> &mut [E::Unit] {
        unsafe {
            let mut len = 0;
            let mut cur = ptr as *mut E::Unit as *const E::Unit;

            while !(*cur).is_zero() {
                len += 1;
                cur = cur.offset(1);
            }

            ::std::slice::from_raw_parts_mut(ptr as *mut E::Unit, len)
        }
    }

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
        unsafe {
            &*((*owned) as *mut E::Unit as *const E::Unit)
        }
    }

    fn borrow_from_owned_mut<'a>(owned: &mut Self::Owned) -> &mut Self::RefTarget {
        unsafe {
            &mut *((*owned) as *mut E::Unit)
        }
    }

    fn as_ffi_ptr(ptr: &Self::RefTarget) -> Self::FfiPtr {
        unsafe {
            mem::transmute::<_, _>(ptr)
        }
    }

    fn as_ffi_ptr_mut(ptr: &mut Self::RefTarget) -> Self::FfiMutPtr {
        unsafe {
            mem::transmute::<_, _>(ptr)
        }
    }
}

impl<E, A> StructureAlloc<E, A> for ZeroTerm where E: Encoding, A: Allocator<Pointer=*mut ()> {
    fn alloc_owned(units: &[E::Unit]) -> Result<Self::Owned, A::AllocError> {
        unsafe {
            // TODO: check for earlier NUL; fail if it isn't at the end.
            let add_term = !(units.len() > 0 && units[units.len()-1].is_zero());

            // +1 for the terminator.
            let total_u = units.len().checked_add(if add_term {1} else {0})
                .ok_or_else(A::AllocError::overflow)?;
            let unit_b = mem::size_of::<E::Unit>();
            let total_b = total_u.checked_mul(unit_b)
                .ok_or_else(A::AllocError::overflow)?;

            let ptr = A::alloc_bytes(total_b, mem::align_of::<E::Unit>())?;
            {
                let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);

                s[..units.len()].copy_from_slice(units);
                s[total_u-1] = E::Unit::zero();
            }

            Ok(ptr)
        }
    }

    fn free_owned(ptr: &mut Self::Owned) {
        unsafe {
            A::free(*ptr, mem::align_of::<E::Unit>());
        }
    }
}

impl<E> StructureDefault<E> for ZeroTerm where E: Encoding {
    fn default<'a>() -> &'a Self::RefTarget {
        unsafe {
            mem::transmute::<*const E::Unit, _>(E::static_zeroes().as_ptr())
        }
    }
}

unsafe impl<E> OwnershipTransfer<E> for ZeroTerm where E: Encoding {
    type OwnedFfiPtr = *mut E::FfiUnit;

    unsafe fn owned_from_ffi_ptr(ptr: Self::OwnedFfiPtr) -> Option<Self::Owned> {
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut ())
        }
    }

    unsafe fn into_ffi_ptr(ptr: &mut Self::Owned) -> Self::OwnedFfiPtr {
        let r = (*ptr) as *mut E::FfiUnit;
        *ptr = ptr::null_mut();
        r
    }
}

impl<E> ZeroTerminated<E> for ZeroTerm where E: Encoding {
    fn slice_units_with_term(ptr: &Self::RefTarget) -> &[E::Unit] {
        unsafe {
            let mut len = 1;
            let mut cur = ptr as *const E::Unit;

            while !(*cur).is_zero() {
                len += 1;
                cur = cur.offset(1);
            }

            ::std::slice::from_raw_parts(ptr as *const E::Unit, len)
        }
    }
}

// pub struct Prefix;

// impl<E> Structure<E> for Prefix where E: Encoding {
//     type Owned = *mut ();
//     type RefTarget = E::Unit;

//     type FfiPtr = *const E::FfiUnit;
//     type FfiMutPtr = *mut E::FfiUnit;

//     fn debug_prefix() -> &'static str { "P" }

//     unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
//         mem::transmute::<TODO, TODO>(ptr)
//     }

//     fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit] {
//         unsafe {
//             let len = *(ptr as *const E::Unit as *const usize).offset(-1);
//             ::std::slice::from_raw_parts(ptr as *const E::Unit, len)
//         }
//     }

//     fn alloc_owned<A>(units: &[E::Unit]) -> Self::Owned where A: Allocator<Pointer=*mut ()> {
//         unsafe {
//             // +1 for the terminator.
//             let total_u = units.len();
//             let units_b = total_u.checked_mul(mem::size_of::<E::Unit>()).expect(here!());
//             let total_b = units_b.checked_add(mem::size_of::<usize>()).expect(here!());

//             let ptr = A::alloc_bytes(total_b, mem::align_of::<usize>());
//             *(ptr as *mut usize) = total_u;
//             let ptr = (ptr as *mut usize).offset(1) as *mut ();
//             {
//                 let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);
//                 s.copy_from_slice(units);
//             }

//             ptr
//         }
//     }

//     fn free_owned<A>(ptr: &mut Self::Owned) where A: Allocator<Pointer=*mut ()> {
//         unsafe {
//             let ptr = (*ptr as *mut usize).offset(-1) as *mut ();
//             A::free(ptr, mem::align_of::<usize>());
//         }
//     }

//     fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
//         unsafe {
//             &*((*owned) as *mut E::Unit as *const E::Unit)
//         }
//     }
// }

// impl<E> ZeroTerminated<E> for Prefix where E: Encoding {
//     fn slice_units_with_term(ptr: &Self::RefTarget) -> &[E::Unit] {
//         unsafe {
//             let len = *(ptr as *const E::Unit as *const usize).offset(-1);
//             ::std::slice::from_raw_parts(ptr as *const E::Unit, len + 1)
//         }
//     }
// }

/**
Strings represented by a pair consisting of a pointer to the first unit, and the number of units stored in a pointer-sized unsigned integer.

This is similar to the representation used by Rust for slices.
*/
pub enum Slice {}

impl<E> Structure<E> for Slice where E: Encoding {
    type Owned = (*mut (), usize);
    type RefTarget = [E::Unit];

    type FfiPtr = (*const E::FfiUnit, usize);
    type FfiMutPtr = (*mut E::FfiUnit, usize);

    fn debug_prefix() -> &'static str { "S" }

    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
        let (ptr, len) = ptr;
        if ptr.is_null() {
            None
        } else {
            Some(::std::slice::from_raw_parts(ptr as *const E::Unit, len))
        }
    }

    unsafe fn borrow_from_ffi_ptr_mut<'a>(ptr: Self::FfiMutPtr) -> Option<&'a mut Self::RefTarget> {
        let (ptr, len) = ptr;
        if ptr.is_null() {
            None
        } else {
            Some(::std::slice::from_raw_parts_mut(ptr as *mut E::Unit, len))
        }
    }

    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit] {
        ptr
    }

    fn slice_units_mut(ptr: &mut Self::RefTarget) -> &mut [E::Unit] {
        ptr
    }

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
        unsafe {
            slice::from_raw_parts(owned.0 as *const () as *const E::Unit, owned.1)
        }
    }

    fn borrow_from_owned_mut<'a>(owned: &mut Self::Owned) -> &mut Self::RefTarget {
        unsafe {
            slice::from_raw_parts_mut(owned.0 as *mut () as *mut E::Unit, owned.1)
        }
    }

    fn as_ffi_ptr(ptr: &Self::RefTarget) -> Self::FfiPtr {
        (ptr.as_ptr() as *const E::FfiUnit, ptr.len())
    }

    fn as_ffi_ptr_mut(ptr: &mut Self::RefTarget) -> Self::FfiMutPtr {
        (ptr.as_mut_ptr() as *mut E::FfiUnit, ptr.len())
    }
}

impl<E, A> StructureAlloc<E, A> for Slice where E: Encoding, A: Allocator<Pointer=*mut ()> {
    fn alloc_owned(units: &[E::Unit]) -> Result<Self::Owned, A::AllocError> {
        unsafe {
            let total_u = units.len();
            let unit_b = mem::size_of::<E::Unit>();
            let total_b = total_u.checked_mul(unit_b)
                .ok_or_else(A::AllocError::overflow)?;

            let ptr = A::alloc_bytes(total_b, mem::align_of::<E::Unit>())?;
            {
                let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);
                s.copy_from_slice(units);
            }

            Ok((ptr as *mut (), total_u))
        }
    }

    fn free_owned(&mut (ptr, _): &mut Self::Owned) {
        unsafe {
            A::free(ptr, mem::align_of::<E::Unit>());
        }
    }
}

impl<E> StructureDefault<E> for Slice where E: Encoding {
    fn default<'a>() -> &'a Self::RefTarget {
        &[]
    }
}

impl KnownLength for Slice {}

unsafe impl<E> OwnershipTransfer<E> for Slice where E: Encoding {
    type OwnedFfiPtr = (*mut E::FfiUnit, usize);

    unsafe fn owned_from_ffi_ptr((ptr, len): Self::OwnedFfiPtr) -> Option<Self::Owned> {
        if ptr.is_null() {
            None
        } else {
            Some((ptr as *mut (), len))
        }
    }

    unsafe fn into_ffi_ptr(ptr: &mut Self::Owned) -> Self::OwnedFfiPtr {
        let (tptr, tlen) = *ptr;
        *ptr = (ptr::null_mut(), 0);
        (tptr as *mut E::FfiUnit, tlen)
    }
}
