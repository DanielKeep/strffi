use std::mem;
use encoding::{TranscodeTo, WUnit};
pub use super::WcToUniError;

impl<It> TranscodeTo<CheckedUnicode> for UnitIter<Wide, It> where It: Iterator<Item=WUnit> {
    type Iter = WcToUniIter<It>;
    type Error = WcToUniError;

    fn transcode(self) -> Self::Iter {
        WcToUniIter::new(self.into_iter())
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
