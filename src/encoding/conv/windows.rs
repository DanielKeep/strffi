use std::mem;
use encoding::{TranscodeTo, WUnit};
pub use super::WcToUniError;

impl<'a> TranscodeTo<char> for &'a [WUnit] {
    type Iter = WcToUniIter2<::std::iter::Cloned<::std::slice::Iter<'a, WUnit>>>;
    type Error = WcToUniError;

    fn transcode(self) -> Self::Iter {
        WcToUniIter2::new(self.iter().cloned())
    }
}

pub struct WcToUniIter2<It> {
    at: usize,
    iter: Option<It>,
}

impl<It> WcToUniIter2<It> {
    pub fn new(iter: It) -> WcToUniIter2<It> {
        WcToUniIter2 {
            at: 0,
            iter: Some(iter),
        }
    }
}

impl<It> Iterator for WcToUniIter2<It> where It: Iterator<Item=WUnit> {
    type Item = Result<char, WcToUniError>;

    fn next(&mut self) -> Option<Self::Item> {
        match {
            match self.iter.as_mut() {
                Some(iter) => iter.next(),
                None => None,
            }
        } {
            None => None,
            Some(cu0) => {
                let r = match cu0.0 as u16 {
                    cu0 @ 0x0000 ... 0xd7ff | cu0 @ 0xe000 ... 0xffff => {
                        self.at += 1;

                        unsafe {
                            let cp = cu0 as u32;
                            let c = mem::transmute::<_, char>(cp);
                            c
                        }
                    },
                    0xdc00 ... 0xdfff => {
                        self.iter = None;
                        return Some(Err(WcToUniError::InvalidAt(self.at)));
                    },
                    cu0 /* @ 0xd800 ... 0xdb00 */ => {
                        let cu1 = match {
                            match self.iter.as_mut() {
                                Some(iter) => iter.next(),
                                None => None,
                            }
                        } {
                            Some(cu1) => cu1.0 as u16,
                            None => {
                                self.iter = None;
                                return Some(Err(WcToUniError::Incomplete));
                            }
                        };

                        if !(0xdc00 <= cu1 && cu1 <= 0xdfff) {
                            self.iter = None;
                            return Some(Err(WcToUniError::InvalidAt(self.at)));
                        }

                        self.at += 2;

                        unsafe {
                            let hi = (cu0 & 0x3ff) as u32;
                            let lo = (cu1 & 0x3ff) as u32;
                            let cp = 0x10000 + ((hi << 10) | lo);
                            let c = mem::transmute::<_, char>(cp);
                            c
                        }
                    },
                };

                Some(Ok(r))
            }
        }
    }
}
