use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use alloc::Allocator;
use encoding::{Encoding, TranscodeTo, CheckedUnicode};
use structure::{Structure, Slice};
use util::{TrapErrExt, Utf8EncodeExt};

pub struct SeStr<S, E> where S: Structure<E>, E: Encoding {
    data: S::RefTarget,
}

impl<E> SeStr<Slice, E> where E: Encoding {
    pub fn new(units: &[E::Unit]) -> &Self {
        unsafe {
            mem::transmute_copy::<&[E::Unit], &Self>(&units)
        }
    }
}

impl<S, E> SeStr<S, E> where S: Structure<E>, E: Encoding {
    pub unsafe fn from_ptr<'a>(ptr: S::FfiPtr) -> &'a Self {
        mem::transmute(S::borrow_from_ffi_ptr(ptr))
    }

    pub fn as_units(&self) -> &[E::Unit] {
        S::slice_units(&self.data)
    }

    pub fn transcode_to<U, F, A>(&self) -> SeaString<U, F, A>
    where
        U: Structure<F>,
        F: Encoding,
        A: Allocator,
        for <'a> &'a [E::Unit]: TranscodeTo<F::Unit>,
    {
        let units: Result<Vec<_>, _> = self.transcode_to_iter::<F>().collect();
        let units = units.expect(here!());
        SeaString::from(&units[..])
    }

    pub fn transcode_to_iter<'a, F>(&'a self) -> <&'a [E::Unit] as TranscodeTo<F::Unit>>::Iter
    where
        F: Encoding,
        &'a [E::Unit]: TranscodeTo<F::Unit>,
    {
        self.as_units().transcode()
    }

    pub fn to_owned<A>(&self) -> SeaString<S, E, A> where A: Allocator {
        SeaString::from(self.as_units())
    }

    pub fn as_slice(&self) -> &SeStr<Slice, E> {
        SeStr::new(self.as_units())
    }

    pub fn into_string(&self) -> String
    where
        for<'a> &'a [E::Unit]: TranscodeTo<char>,
    {
        let mut err = Ok(());
        let units: Vec<_> = self
            .transcode_to_iter::<CheckedUnicode>()
            .trap_err(&mut err)
            .encode_utf8()
            .collect();
        err.expect(here!()); // TODO
        let s = unsafe { String::from_utf8_unchecked(units) };
        s
    }
}

pub struct SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    owned: S::Owned,
    _marker: PhantomData<A>,
}

/*impl<S, E, A> SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
}*/

impl<S, E, A> AsRef<SeStr<S, E>> for SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    fn as_ref(&self) -> &SeStr<S, E> {
        unsafe {
            mem::transmute(S::borrow_from_owned(&self.owned))
        }
    }
}

impl<S, E, A> Deref for SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    type Target = SeStr<S, E>;

    fn deref(&self) -> &SeStr<S, E> {
        unsafe {
            mem::transmute(S::borrow_from_owned(&self.owned))
        }
    }
}

impl<S, E, A> Drop for SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    fn drop(&mut self) {
        println!("-- SeaString::drop(_)");
        S::free_owned::<A>(&mut self.owned);
    }
}

impl<'a, S, E, A> From<&'a [E::Unit]> for SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    fn from(value: &'a [E::Unit]) -> Self {
        let owned = S::alloc_owned::<A>(value);
        SeaString {
            owned: owned,
            _marker: PhantomData,
        }
    }
}

impl<S, E, A> FromIterator<E::Unit> for SeaString<S, E, A>
where
    S: Structure<E>,
    E: Encoding,
    A: Allocator,
{
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item=E::Unit> {
        let units: Vec<_> = iter.into_iter().collect();
        SeaString::from(&units[..])
    }
}
