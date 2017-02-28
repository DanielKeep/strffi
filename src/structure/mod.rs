use std::mem;
use std::slice;
use alloc::Allocator;
use encoding::{Encoding, Unit};

pub trait Structure<E>: Sized where E: Encoding {
    type Owned;
    type RefTarget: ?Sized;

    type FfiPtr;
    type FfiMutPtr;

    type AllocAlign;

    /*
    Why do we return an optional borrow rather than a pointer?  Because a null `*const [T]` isn't something Rust really wants to have as a thing, mostly because it raises the question of what the length is supposed to be.  Maybe "0" is the obvious answer.  What's *less* obvious is what the second word for `*const Trait` is supposed to be.

    So, rather than deal with that, or potentially invoke nasal demons, we just do something Rust *is* OK with.
    */
    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget>;

    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit];

    fn alloc_owned<A>(units: &[E::Unit]) -> Self::Owned where A: Allocator;
    fn free_owned<A>(ptr: &mut Self::Owned) where A: Allocator;

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget;
}

pub struct ZeroTerm;

impl<E> Structure<E> for ZeroTerm where E: Encoding {
    type Owned = *mut ();
    type RefTarget = E::Unit;

    type FfiPtr = *const E::FfiUnit;
    type FfiMutPtr = *mut E::FfiUnit;

    type AllocAlign = E::Unit;

    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
        mem::transmute(ptr)
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

    fn alloc_owned<A>(units: &[E::Unit]) -> Self::Owned where A: Allocator {
        println!("-- ZeroTerm::alloc_owned(_)");
        unsafe {
            // TODO: check for earlier NUL; fail if it isn't at the end.
            let add_term = !(units.len() > 0 && units[units.len()-1].is_zero());

            // +1 for the terminator.
            let total_u = units.len().checked_add(if add_term {1} else {0}).expect(here!());
            let unit_b = mem::size_of::<E::Unit>();
            let total_b = total_u.checked_mul(unit_b).expect(here!());

            let ptr = A::alloc_bytes(total_b, mem::align_of::<E::Unit>());
            {
                let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);

                s[..units.len()].copy_from_slice(units);
                s[total_u-1] = E::Unit::zero();
            }

            ptr
        }
    }

    fn free_owned<A>(ptr: &mut Self::Owned) where A: Allocator {
        println!("-- ZeroTerm::free_owned(_)");
        unsafe {
            A::free(*ptr, mem::align_of::<E::Unit>());
        }
    }

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
        unsafe {
            &*((*owned) as *mut E::Unit as *const E::Unit)
        }
    }
}

pub struct Prefix;

impl<E> Structure<E> for Prefix where E: Encoding {
    type Owned = *mut ();
    type RefTarget = E::Unit;

    type FfiPtr = *const E::FfiUnit;
    type FfiMutPtr = *mut E::FfiUnit;

    type AllocAlign = usize;

    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
        mem::transmute(ptr)
    }

    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit] {
        unsafe {
            let len = *(ptr as *const E::Unit as *const usize).offset(-1);
            ::std::slice::from_raw_parts(ptr as *const E::Unit, len)
        }
    }

    fn alloc_owned<A>(units: &[E::Unit]) -> Self::Owned where A: Allocator {
        println!("-- Prefix::alloc_owned(_)");
        unsafe {
            // +1 for the terminator.
            let total_u = units.len();
            let units_b = total_u.checked_mul(mem::size_of::<E::Unit>()).expect(here!());
            let total_b = units_b.checked_add(mem::size_of::<usize>()).expect(here!());

            let ptr = A::alloc_bytes(total_b, mem::align_of::<usize>());
            *(ptr as *mut usize) = total_u;
            let ptr = (ptr as *mut usize).offset(1) as *mut ();
            {
                let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);
                s.copy_from_slice(units);
            }

            ptr
        }
    }

    fn free_owned<A>(ptr: &mut Self::Owned) where A: Allocator {
        println!("-- Prefix::free_owned(_)");
        unsafe {
            let ptr = (*ptr as *mut usize).offset(-1) as *mut ();
            A::free(ptr, mem::align_of::<usize>());
        }
    }

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
        unsafe {
            &*((*owned) as *mut E::Unit as *const E::Unit)
        }
    }
}

pub struct Slice;

impl<E> Structure<E> for Slice where E: Encoding {
    type Owned = (*mut (), usize);
    type RefTarget = [E::Unit];

    type FfiPtr = (*const E::FfiUnit, usize);
    type FfiMutPtr = (*mut E::FfiUnit, usize);

    type AllocAlign = E::Unit;

    unsafe fn borrow_from_ffi_ptr<'a>(ptr: Self::FfiPtr) -> Option<&'a Self::RefTarget> {
        let (ptr, len) = ptr;
        if ptr.is_null() {
            None
        } else {
            Some(::std::slice::from_raw_parts(ptr as *const E::Unit, len))
        }
    }

    fn slice_units(ptr: &Self::RefTarget) -> &[E::Unit] {
        ptr
    }

    fn alloc_owned<A>(units: &[E::Unit]) -> Self::Owned where A: Allocator {
        println!("-- Slice::alloc_owned(_)");
        unsafe {
            let total_u = units.len();
            let unit_b = mem::size_of::<E::Unit>();
            let total_b = total_u.checked_mul(unit_b).expect(here!());

            let ptr = A::alloc_bytes(total_b, mem::align_of::<E::Unit>());
            {
                let s = slice::from_raw_parts_mut(ptr as *mut E::Unit, total_u);
                s.copy_from_slice(units);
            }

            (ptr as *mut (), total_u)
        }
    }

    fn free_owned<A>(&mut (ptr, _): &mut Self::Owned) where A: Allocator {
        println!("-- Slice::free_owned(_)");
        unsafe {
            A::free(ptr, mem::align_of::<E::Unit>());
        }
    }

    fn borrow_from_owned<'a>(owned: &Self::Owned) -> &Self::RefTarget {
        unsafe {
            slice::from_raw_parts(owned.0 as *const () as *const E::Unit, owned.1)
        }
    }
}
