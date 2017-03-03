use std::mem;
use encoding::{TranscodeTo, UnitIter, CheckedUnicode, Wide, WUnit};
pub use super::{NoError, WcToUniError};

impl<It> TranscodeTo<CheckedUnicode> for UnitIter<Wide, It> where It: Iterator<Item=WUnit> {
    type Iter = WcToUniIter<It>;
    type Error = WcToUniError;

    fn transcode(self) -> Self::Iter {
        WcToUniIter::new(self.into_iter())
    }
}

impl<It> TranscodeTo<Wide> for UnitIter<CheckedUnicode, It> where It: Iterator<Item=char> {
    type Iter = UniToWcIter<It>;
    type Error = NoError;

    fn transcode(self) -> Self::Iter {
        UniToWcIter::new(self.into_iter())
    }
}

pub struct WcToUniIter<It> {
    at: usize,
    iter: Option<It>,
}

impl<It> WcToUniIter<It> {
    pub fn new(iter: It) -> WcToUniIter<It> {
        WcToUniIter {
            at: 0,
            iter: Some(iter),
        }
    }
}

pub struct UniToWcIter<It> {
    at: usize,
    buf: Option<WUnit>,
    iter: Option<It>,
}

impl<It> UniToWcIter<It> {
    pub fn new(iter: It) -> UniToWcIter<It> {
        UniToWcIter {
            at: 0,
            buf: None,
            iter: Some(iter),
        }
    }
}

impl<It> Iterator for WcToUniIter<It> where It: Iterator<Item=WUnit> {
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

impl<It> Iterator for UniToWcIter<It> where It: Iterator<Item=char> {
    type Item = Result<WUnit, NoError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(wunit) = self.buf.take() {
            return Some(Ok(wunit));
        }

        let ch = match {
            match self.iter.as_mut() {
                Some(iter) => iter.next(),
                None => return None,
            }
        } {
            Some(ch) => ch,
            None => {
                self.iter = None;
                return None
            },
        };

        let mut utf16 = [0; 2];
        let utf16 = ch.encode_utf16(&mut utf16[..]);
        self.buf = utf16.get(1).map(|&u| WUnit(u));
        Some(Ok(WUnit(utf16[0])))
    }
}
