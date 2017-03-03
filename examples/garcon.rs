extern crate libc;
extern crate strffi;

macro_rules! here { () => { &format!(concat!(file!(), ":{:?}"), line!()) } }

use std::ffi::CStr;
use std::ptr;

#[cfg(windows)] // cp1252
const WORD: &'static [u8] = b"\x93g\xaar\xe7on\x94\0";
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
    extern "C" {
        fn _getmbcp() -> libc::c_int;
    }
    extern "system" {
        fn GetACP() -> u32;
        fn GetOEMCP() -> u32;
        fn GetConsoleCP() -> u32;
        fn GetConsoleOutputCP() -> u32;
    }

    unsafe {
        let result = libc::setlocale(libc::LC_ALL, ptr::null());
        println!("locale: {:?}", CStr::from_ptr(result));

        let cur_mbcp = _getmbcp();
        println!("current MB cp: {:?}", cur_mbcp);

        let cur_acp = GetACP();
        println!("current ANSI cp: {:?}", cur_acp);

        let cur_ocp = GetOEMCP();
        println!("current OEM cp: {:?}", cur_ocp);

        let cur_cicp = GetConsoleCP();
        println!("current console input cp: {:?}", cur_cicp);

        let cur_cocp = GetConsoleOutputCP();
        println!("current console output cp: {:?}", cur_cocp);

        // let result = libc::setlocale(libc::LC_ALL, b".28591\0".as_ptr() as *const _);
        let result = libc::setlocale(libc::LC_ALL, b".1252\0".as_ptr() as *const _);
        if result.is_null() {
            panic!("could not set locale");
        }
        println!("locale: {:?}", CStr::from_ptr(result));

        let cur_mbcp = _getmbcp();
        println!("current MB cp: {:?}", cur_mbcp);

        let cur_acp = GetACP();
        println!("current ANSI cp: {:?}", cur_acp);

        let cur_ocp = GetOEMCP();
        println!("current OEM cp: {:?}", cur_ocp);

        let cur_cicp = GetConsoleCP();
        println!("current console input cp: {:?}", cur_cicp);

        let cur_cocp = GetConsoleOutputCP();
        println!("current console output cp: {:?}", cur_cocp);

        if cur_acp != 1252 {
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
        use strffi::{ZMbStr, ZWCString};
        let zmbstr = unsafe { ZMbStr::from_ptr(WORD.as_ptr() as *const _).expect(here!()) };
        println!("zmbstr: {:?}", zmbstr);
        {
            let rstr = zmbstr.into_string().expect(here!());
            println!("via strffi (zmb->r): {:?}", rstr);
        }
        let zwrstr: ZWCString = zmbstr.transcode_to().expect(here!());
        println!("zwrstr: {:?}", zwrstr);
        {
            let rstr = zwrstr.into_string().expect(here!());
            println!("via strffi (zmb->zw->r): {:?}", rstr);
        }
    }
}
