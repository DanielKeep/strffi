/*!
Allocation types and traits.
*/
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::mem;
pub use self::rust::Rust;

use libc::{self, c_void};

/**
Abstracts over different memory allocators.

In practice, this will be implemented by a marker type (which are not intended to actually be instantiated anywhere), along with a concrete type that implements `Unit`, and likely at least one implementation of `TranscodeTo`.
*/
pub trait Allocator {
    /**
    The type of errors that can occur during allocation.
    */
    type AllocError: AllocatorError + 'static;

    /**
    The pointer type that this allocator produces.

    Unless the allocator has special restrictions that limit the kinds of string structures it can allocate, this type should be `*mut ()`.
    */
    type Pointer;

    /**
    Allocate the specified number of bytes, with the specified alignment.
    */
    fn alloc_bytes(bytes: usize, align: usize) -> Result<Self::Pointer, Self::AllocError>;

    /**
    Free an allocation.

    Although this method specifies the alignment the pointer was allocated with, it does *not* specify the length.  This is because it is not always possible to determine the original length of the allocation.  If your allocator needs to know the length of the allocation, you will need to hide the length as part of the allocation itself and recover the information on deallocation.
    */
    unsafe fn free(ptr: Self::Pointer, align: usize);

    /**
    Returns a string which can be used to uniquely identify this allocator in debug output.

    This string should *preferably* be short, reasonably evocative, unique, and a single `Camelword`, although nothing will break if this is not done.

    For context, the debug representation of `SeaString` involves concatenating the debug prefixes of the structure, encoding, and allocator together.
    */
    fn debug_prefix() -> &'static str;
}

/**
This trait defines the required interface for allocation errors.
*/
pub trait AllocatorError: StdError {
    /**
    Construct an error indicating that an overflow occurred when computing the size of the allocation.

    This exists to allow string structures to safely indicate that the size of an allocation exceeded some intrinsic limit.
    */
    fn overflow() -> Self;
}

/**
A general allocation error.
*/
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllocError {
    Failed,
    CannotAlign,
    SizeOverflow,
}

impl AllocatorError for AllocError {
    fn overflow() -> Self {
        AllocError::SizeOverflow
    }
}

impl Display for AllocError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.description())
    }
}

impl StdError for AllocError {
    fn description(&self) -> &'static str {
        match *self {
            AllocError::Failed => "failed to allocate memory",
            AllocError::CannotAlign => "cannot satisfy requested alignment",
            AllocError::SizeOverflow => "overflow while computing size",
        }
    }
}

/**
Represents the C runtime heap allocator.
*/
pub enum Malloc {}

impl Allocator for Malloc {
    type AllocError = AllocError;
    type Pointer = *mut ();

    fn alloc_bytes(bytes: usize, align: usize) -> Result<*mut (), AllocError> {
        // println!("-- Malloc::alloc_bytes({:?}, {:?})", bytes, _align);
        unsafe {
            // A conservative guess.
            if align > mem::align_of::<usize>() {
                return Err(AllocError::CannotAlign);
            }

            let ptr = libc::calloc(bytes, 1);
            if ptr.is_null() {
                Err(AllocError::Failed)
            } else {
                Ok(ptr as *mut ())
            }
        }
    }

    unsafe fn free(ptr: *mut (), _align: usize) {
        // println!("-- Malloc::free(_, {:?})", _align);
        if !ptr.is_null() {
            libc::free(ptr as *mut c_void);
        }
    }

    fn debug_prefix() -> &'static str { "C" }
}

#[cfg(all(feature="nightly", feature="nightly-alloc"))]
mod rust {
    use std::cmp;
    use std::mem;
    use rust_alloc::heap;
    use super::{Allocator, AllocError};

    /**
    Represents the Rust runtime heap allocator.
    */
    pub enum Rust {}

    impl Allocator for Rust {
        type AllocError = AllocError;
        type Pointer = *mut ();

        fn alloc_bytes(bytes: usize, align: usize) -> Result<*mut (), AllocError> {
            // println!("-- Rust::alloc_bytes({:?}, {:?})", bytes, align);
            unsafe {
                let align = cmp::min(mem::align_of::<usize>(), align);
                let bytes = bytes.checked_add(align).ok_or(AllocError::SizeOverflow)?;

                let ptr = heap::allocate(bytes, align);
                if ptr.is_null() {
                    return Err(AllocError::Failed);
                }

                // Save the length for later.
                *(ptr as *mut usize) = bytes;
                let ptr = ptr.offset(align as isize);

                Ok(ptr as *mut ())
            }
        }

        unsafe fn free(ptr: *mut (), align: usize) {
            // println!("-- Rust::free(_, {:?})", align);
            if !ptr.is_null() {
                let align = cmp::min(mem::align_of::<usize>(), align);

                let ptr = ptr.offset(-(align as isize));
                let bytes = *(ptr as *mut usize);

                heap::deallocate(ptr as *mut u8, bytes, align);
            }
        }

        fn debug_prefix() -> &'static str { "R" }
    }
}

#[cfg(not(all(feature="nightly", feature="nightly-alloc")))]
mod rust {
    use super::{Allocator, AllocError};

    /**
    Represents the Rust runtime heap allocator.
    */
    pub enum Rust {}

    impl Allocator for Rust {
        type AllocError = AllocError;
        type Pointer = *mut ();

        fn alloc_bytes(bytes: usize, align: usize) -> Result<*mut (), AllocError> {
            // println!("-- Rust::alloc_bytes({:?}, {:?})", bytes, align);
            unsafe {
                if align > 8 {
                    return Err(AllocError::CannotAlign);
                }

                let words = (bytes + 15) / 8;
                let vec = vec![0u64; words];
                vec[0] = bytes as u64;
                let arr = vec.into_boxed_slice();
                let ptr = arr.into_raw().as_ptr().offset(1);
                Ok(ptr as *mut ())
            }
        }

        unsafe fn free(ptr: *mut (), align: usize) {
            // println!("-- Rust::free(_, {:?})", align);
            if !ptr.is_null() {
                let ptr = (ptr as *mut u64).offset(-1);
                let bytes = (*ptr) as usize;
                let slice = slice::from_raw_parts_mut(ptr, bytes) as *mut _;
                let arr = Box::from_raw(slice);
                drop(arr);
            }
        }

        fn debug_prefix() -> &'static str { "R" }
    }
}
