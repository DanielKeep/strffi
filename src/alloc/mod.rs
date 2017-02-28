pub use self::rust::Rust;

use libc::{self, c_void};

pub trait Allocator {
    fn alloc_bytes(bytes: usize, align: usize) -> *mut ();
    unsafe fn free(ptr: *mut (), align: usize);
}

pub enum Malloc {}

impl Allocator for Malloc {
    fn alloc_bytes(bytes: usize, _align: usize) -> *mut () {
        println!("-- Malloc::alloc_bytes({:?}, {:?})", bytes, _align);
        unsafe {
            let ptr = libc::calloc(bytes, 1);
            if ptr.is_null() {
                panic!("o noes!");
            }
            ptr as *mut ()
        }
    }

    unsafe fn free(ptr: *mut (), _align: usize) {
        println!("-- Malloc::free(_, {:?})", _align);
        if !ptr.is_null() {
            libc::free(ptr as *mut c_void);
        }
    }
}

#[cfg(all(feature="nightly", feature="nightly-alloc"))]
mod rust {
    use std::cmp;
    use std::mem;
    use rust_alloc::heap;
    use super::Allocator;

    pub enum Rust {}

    impl Allocator for Rust {
        fn alloc_bytes(bytes: usize, align: usize) -> *mut () {
            println!("-- Rust::alloc_bytes({:?}, {:?})", bytes, align);
            unsafe {
                let align = cmp::min(mem::align_of::<usize>(), align);
                let bytes = bytes.checked_add(align).expect(here!());

                let ptr = heap::allocate(bytes, align);
                if ptr.is_null() {
                    panic!("o noes!");
                }

                // Save the length for later.
                *(ptr as *mut usize) = bytes;
                let ptr = ptr.offset(align as isize);

                ptr as *mut ()
            }
        }

        unsafe fn free(ptr: *mut (), align: usize) {
            println!("-- Rust::free(_, {:?})", align);
            if !ptr.is_null() {
                let align = cmp::min(mem::align_of::<usize>(), align);

                let ptr = ptr.offset(-(align as isize));
                let bytes = *(ptr as *mut usize);

                heap::deallocate(ptr as *mut u8, bytes, align);
            }
        }
    }
}

#[cfg(not(all(feature="nightly", feature="nightly-alloc")))]
mod rust {
    use super::Allocator;

    pub enum Rust {}

    impl Allocator for Rust {
        fn alloc_bytes(bytes: usize, align: usize) -> *mut () {
            println!("-- Rust::alloc_bytes({:?}, {:?})", bytes, align);
            unsafe {
                if align > 8 {
                    panic!("align too high: {:?}", align);
                }

                let words = (bytes + 15) / 8;
                let vec = vec![0u64; words];
                vec[0] = bytes as u64;
                let arr = vec.into_boxed_slice();
                let ptr = arr.into_raw().as_ptr().offset(1);
                ptr as *mut ()
            }
        }

        unsafe fn free(ptr: *mut (), align: usize) {
            println!("-- Rust::free(_, {:?})", align);
            if !ptr.is_null() {
                let ptr = (ptr as *mut u64).offset(-1);
                let bytes = (*ptr) as usize;
                let slice = slice::from_raw_parts_mut(ptr, bytes) as *mut _;
                let arr = Box::from_raw(slice);
                drop(arr);
            }
        }
    }
}
