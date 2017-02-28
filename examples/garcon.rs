extern crate libc;
extern crate strffi;

use std::ffi::CStr;
use std::ptr;

#[cfg(windows)] // cp1252
const WORD: &'static [u8] = b"g\xaar\xe7on\0";
#[cfg(unix)] // utf-8
const WORD: &'static [u8] = b"g\xc2\xaar\xc3\xa7on\0";

#[cfg(unix)]
fn locale_init() {
    unsafe {
        let result = libc::setlocale(libc::LC_ALL, ptr::null());
        println!("locale: {:?}", CStr::from_ptr(result));

        let result = libc::setlocale(libc::LC_ALL, b"C.UTF-8\0".as_ptr() as *const _);
        if result.is_null() {
            panic!("could not set locale");
        }

        let result = CStr::from_ptr(result);
        println!("locale: {:?}", result);
    }
}

#[cfg(windows)]
fn locale_init() {
    extern "system" {
        fn GetACP() -> u32;
    }

    unsafe {
        let result = libc::setlocale(libc::LC_ALL, ptr::null());
        println!("locale: {:?}", CStr::from_ptr(result));

        let cur_cp = GetACP();
        println!("current codepage: {:?}", cur_cp);

        let result = libc::setlocale(libc::LC_ALL, b".1252\0".as_ptr() as *const _);
        if result.is_null() {
            panic!("could not set locale");
        }
        println!("locale: {:?}", CStr::from_ptr(result));

        let cur_cp = GetACP();
        println!("current codepage: {:?}", cur_cp);

        if cur_cp != 1252 {
            panic!("Requires CP1252 on Windows.");
        }
    }
}

fn main() {
    locale_init();

    println!("utf8: {:?}", "gªrçon");
    println!("utf8 bytes: {:?}", "gªrçon".as_bytes());

    if true {
        let cstr = unsafe { CStr::from_ptr(WORD.as_ptr() as *const _) };
        println!("cstr: {:?}", cstr);
        match cstr.to_str() {
            Ok(rstr) => println!("via cstr: {:?}", rstr),
            Err(err) => println!("couldn't convert CStr to str: {}", err),
        }
    }
    if true {
        use strffi::{ZMbStr, ZWRString};
        let mbzstr = unsafe { ZMbStr::from_ptr(WORD.as_ptr() as *const _) };
        {
            let rstr = mbzstr.into_string();
            println!("via strffi (zmb->r): {:?}", rstr);
        }
        let wzcstr: ZWRString = mbzstr.transcode_to();
        {
            let rstr = wzcstr.into_string();
            println!("via strffi (zmb->zw->r): {:?}", rstr);
        }
    }
}
