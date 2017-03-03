#![cfg(target_os="linux")]
extern crate libc;
extern crate strffi;

macro_rules! here { () => { &format!(concat!(file!(), ":{:?}"), line!()) } }

use strffi::{ZMbStr, ZMbCString, ZWCString, ZWStr};

fn set_utf8() {
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

    set_utf8();

    let zmbstr = unsafe { ZMbStr::from_ptr(WORD_MB.as_ptr() as *const _).expect(here!()) };
    let zwstr = unsafe { ZWStr::from_ptr(WORD_W.as_ptr() as *const _).expect(here!()) };
    {
        let rstr = zmbstr.into_string().expect(here!());
        assert_eq!(&rstr, WORD);
    }
    {
        let rstr = zwstr.into_string().expect(here!());
        assert_eq!(&rstr, WORD);
    }
    {
        let zwcstr: ZWCString = zmbstr.transcode_to().expect(here!());
        assert_eq!(&zwcstr, zwstr);
        let rstr = zwcstr.into_string().expect(here!());
        assert_eq!(&rstr, WORD);
    }
    {
        let zmbcstr: ZMbCString = zwstr.transcode_to().expect(here!()).into();
        assert_eq!(&zmbcstr, zmbstr);
        let rstr = zmbcstr.into_string().expect(here!());
        assert_eq!(&rstr, WORD);
    }
    {
        let zmbcstr = ZMbCString::from_str(WORD).expect(here!());
        assert_eq!(&zmbcstr, zmbstr);
    }
    {
        let zwcstr = ZWCString::from_str(WORD).expect(here!());
        assert_eq!(&zwcstr, zwstr);
    }
}
