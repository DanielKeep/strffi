use std::mem;
use encoding::{TranscodeTo, UnitIter, Wide, WUnit, CheckedUnicode};
use encoding::conv::NoError;
pub use super::WcToUniError;

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
    pub fn new(iter: It) -> Self {
        WcToUniIter {
            at: 0,
            iter: Some(iter),
        }
    }
}

pub struct UniToWcIter<It> {
    iter: Option<It>,
}

impl<It> UniToWcIter<It> {
    pub fn new(iter: It) -> Self {
        UniToWcIter {
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
                None => return None,
            }
        } {
            None => None,
            Some(cp) => {
                let cp = cp.0 as u32;
                let cp = match cp {
                    0x000000 ... 0x02FFFF => cp,
                    0x030000 ... 0x0DFFFF => {
                        self.iter = None;
                        return Some(Err(WcToUniError::InvalidAt(self.at)));
                    },
                    0x0E0000 ... 0x10FFFF => cp,
                    _ => {
                        self.iter = None;
                        return Some(Err(WcToUniError::InvalidAt(self.at)));
                    }
                };

                self.at += 1;

                unsafe {
                    let c = mem::transmute::<u32, char>(cp);
                    Some(Ok(c))
                }
            }
        }
    }
}

impl<It> Iterator for UniToWcIter<It> where It: Iterator<Item=char> {
    type Item = Result<WUnit, NoError>;

    fn next(&mut self) -> Option<Self::Item> {
        match {
            match self.iter.as_mut() {
                Some(iter) => iter.next(),
                None => return None,
            }
        } {
            None => None,
            Some(cp) => {
                // Now, pay attention, because this is *very* complicated.
                Some(Ok(WUnit(cp as u32 as i32)))
            }
        }
    }
}
