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
    pub fn new(iter: It) -> Self {
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
