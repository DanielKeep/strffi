#![cfg(target_os="windows")]
extern crate libc;
extern crate strffi;

use strffi::{ZMbStr, ZWCString, ZWStr};

fn set_1252() {
    unsafe {
        let r = libc::setlocale(libc::LC_ALL, b".1252".as_ptr() as *const _);
        assert!(!r.is_null());
    }
}

#[test]
fn test_garcon() {
    const WORD: &'static str = "gªrçon";
    const WORD_MB: &'static [u8] = b"g\xaar\xe7on\0";
    const WORD_W: &'static [u16] = &[0x67, 0xAA, 0x72, 0xE7, 0x6F, 0x6E, 0x00];

    set_1252();

    {
        let zmbstr = unsafe { ZMbStr::from_ptr(WORD_MB.as_ptr() as *const _) };
        let rstr = zmbstr.into_string();
        assert_eq!(&rstr, WORD);
    }
    {
        let zwstr = unsafe { ZWStr::from_ptr(WORD_W.as_ptr() as *const _) };
        let rstr = zwstr.into_string();
        assert_eq!(&rstr, WORD);
    }
    {
        let zmbstr = unsafe { ZMbStr::from_ptr(WORD_MB.as_ptr() as *const _) };
        let zwcstr: ZWCString = zmbstr.transcode_to();
        let rstr = zwcstr.into_string();
        assert_eq!(&rstr, WORD);
    }
}
