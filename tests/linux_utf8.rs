#![cfg(target_os="linux")]
extern crate libc;
extern crate strffi;

use strffi::{ZMbStr, ZWCString, ZWStr};

fn set_1252() {
    unsafe {
        let r = libc::setlocale(libc::LC_ALL, b"C.UTF-8".as_ptr() as *const _);
        assert!(!r.is_null());
    }
}

#[test]
fn test_garcon() {
    const WORD: &'static str = "gªrçon";
    const WORD_MB: &'static [u8] = b"g\xc2\xaar\xc3\xa7on\0";
    const WORD_W: &'static [u32] = &[0x67, 0xAA, 0x72, 0xE7, 0x6F, 0x6E, 0x00];

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
