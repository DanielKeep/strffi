use libc::{c_char, size_t, wchar_t};

// TODO: move into libc

/*
We have no way of knowing what encodings we'll have to deal with, so 16 was chosen as a (hopefully) excessive upper bound.

Keep in mind that there are serious encodings in existence (though probably *not* being used as the C MB encoding) that can require up to *12 bytes* for a single character.

Normally, you would get this from `limits.h`, except it's not even necessarily a compile-time constant.  Bah!
*/
pub const MB_LEN_MAX: usize = 16;

extern "C" {
    pub fn mbrtowc(dest: *mut wchar_t, src: *const c_char, n: size_t, mbs: *mut mbstate_t) -> size_t;
    pub fn wcrtomb(dest: *mut c_char, src: wchar_t, mbs: *mut mbstate_t) -> size_t;
}

#[cfg(all(target_arch="x86", target_os="windows", target_env="gnu"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mbstate_t {
    _data: [u32; 1]
}

#[cfg(all(target_arch="x86_64", target_os="linux", target_env="gnu"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mbstate_t {
    _data: [u32; 2]
}

#[cfg(all(target_arch="x86_64", target_os="windows", target_env="msvc"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mbstate_t {
    _data: [u32; 2]
}
