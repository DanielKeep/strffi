use std::fmt;
use std::iter;
use std::mem;
use std::slice;
use libc::{c_char};
use encoding::{TranscodeTo, MbUnit, WUnit};
use encoding::conv::os::{WcToUniIter2, WcToUniError};
use ffi::{MB_LEN_MAX, mbrtowc, wcrtomb, mbstate_t};
use util::{LiftErrIter, LiftTrapErrIter, LiftErrExt};

impl<'a> TranscodeTo<WUnit> for &'a [MbUnit] {
    type Iter = MbsToWcIter2<iter::Cloned<slice::Iter<'a, MbUnit>>>;
    type Error = MbsToWcError;

    fn transcode(self) -> Self::Iter {
        MbsToWcIter2::new(self.iter().cloned())
    }
}

impl<'a> TranscodeTo<char> for &'a [MbUnit] {
    type Iter = LiftErrIter<
        iter::Map<
            WcToUniIter2<
                LiftTrapErrIter<
                    MbsToWcIter2<iter::Cloned<slice::Iter<'a, MbUnit>>>,
                    MbsToWcError,
                >
            >,
            fn(Result<char, WcToUniError>) -> Result<char, MbsToUniError>,
        >,
        MbsToWcError,
    >;
    type Error = MbsToUniError;

    fn transcode(self) -> Self::Iter {
        MbsToWcIter2::new(self.iter().cloned())
            .lift_err(|over| WcToUniIter2::new(over)
                .map(map_err as fn(_) -> _))
    }
}

pub struct MbsToWcIter2<It> {
    iter: Option<It>,
    at: usize,
    // buf: [c_char; MB_LEN_MAX],
    // buf_len: u8,
    state: mbstate_t,
}

impl<It> MbsToWcIter2<It> {
    pub fn new(iter: It) -> Self {
        MbsToWcIter2 {
            iter: Some(iter),
            at: 0,
            state: unsafe { mem::zeroed() },
        }
    }
}

pub struct WcsToMbIter<It> {
    iter: Option<It>,
    at: usize,
    buf: [MbUnit; MB_LEN_MAX],
    buf_at: u8,
    buf_len: u8,
    state: mbstate_t,
}

impl<It> WcsToMbIter<It> {
    pub fn new(iter: It) -> Self {
        WcsToMbIter {
            iter: Some(iter),
            at: 0,
            buf: [MbUnit(0); MB_LEN_MAX],
            buf_at: 0,
            buf_len: 0,
            state: unsafe { mem::zeroed() },
        }
    }
}

impl<It> Iterator for MbsToWcIter2<It> where It: Iterator<Item=MbUnit> {
    type Item = Result<WUnit, MbsToWcError>;

    fn next(&mut self) -> Option<Self::Item> {
        let err;

        {
            let mut buf = [0; MB_LEN_MAX];
            let mut buf_len = 0;

            let iter = match self.iter.as_mut() {
                Some(iter) => iter,
                None => return None,
            };

            loop {
                if buf_len == buf.len() {
                    err = MbsToWcError::OutOfBufferAt(self.at);
                    break;
                }

                buf[buf_len] = match {
                    let e = iter.next();
                    e
                } {
                    Some(mbu) => mbu.0,
                    None => {
                        if buf_len == 0 {
                            return None;
                        } else {
                            err = MbsToWcError::Incomplete;
                            break;
                        }
                    },
                };
                buf_len += 1;

                const ILLEGAL: usize = -1isize as usize;
                const INCOMPLETE: usize = -2isize as usize;

                let mut wc = 0;
                let mut state_new = self.state;

                match unsafe {
                    let r = mbrtowc(&mut wc,
                        buf.as_ptr() as *const c_char,
                        buf_len as usize,
                        &mut state_new);
                    r
                } {
                    ILLEGAL => {
                        err = MbsToWcError::InvalidAt(self.at);
                        break;
                    },

                    INCOMPLETE => {
                        // We have to keep pulling new units in until we run out or exhaust the buffer.
                        continue;
                    },

                    _ => (),
                }

                self.at += buf_len as usize;
                self.state = state_new;

                return Some(Ok(WUnit(wc)));
            }
        }

        self.iter = None;
        Some(Err(err))
    }
}

impl<It> Iterator for WcsToMbIter<It> where It: Iterator<Item=WUnit> {
    type Item = Result<MbUnit, WcsToMbError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf_at < self.buf_len {
            let mbu = self.buf[self.buf_at as usize];
            self.buf_at += 1;
            return Some(Ok(mbu));
        }

        // Refresh buffer
        self.buf_at = 0;
        self.buf_len = 0;

        match {
            match self.iter.as_mut() {
                Some(iter) => iter.next(),
                None => return None,
            }
        } {
            None => return None,
            Some(wcu) => {
                unsafe {
                    const ILLEGAL: usize = -1isize as usize;
                    match {
                        wcrtomb(
                            self.buf[..].as_mut_ptr() as *mut c_char,
                            wcu.0,
                            &mut self.state)
                    } {
                        ILLEGAL => {
                            self.iter = None;
                            return Some(Err(WcsToMbError::InvalidAt(self.at)));
                        },
                        0 => {
                            // This... *shouldn't happen.*
                            panic!("wcrtomb wrote no multibyte units for {:?}", wcu);
                        },
                        len if len > MB_LEN_MAX => {
                            // We can *probably* assume memory corruption.
                            panic!("wcrtomb has corrupted memory");
                        },
                        len => {
                            self.at += 1;
                            self.buf_at = 1;
                            self.buf_len = len as u8;
                            return Some(Ok(self.buf[0]));
                        },
                    }
                }
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MbsToWcError {
    InvalidAt(usize),
    Incomplete,
    OutOfBufferAt(usize),
}

impl fmt::Display for MbsToWcError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MbsToWcError::InvalidAt(at) => write!(fmt, "invalid unit at offset {}", at),
            MbsToWcError::Incomplete => write!(fmt, "incomplete unit"),
            MbsToWcError::OutOfBufferAt(at) => write!(fmt, "character too large to transcode at offset {}", at),
        }
    }
}

impl ::std::error::Error for MbsToWcError {
    fn description(&self) -> &str {
        match *self {
            MbsToWcError::InvalidAt(_) => "invalid unit",
            MbsToWcError::Incomplete => "incomplete unit",
            MbsToWcError::OutOfBufferAt(_) => "character too large to transcode",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WcsToMbError {
    InvalidAt(usize),
}

impl fmt::Display for WcsToMbError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WcsToMbError::InvalidAt(at) => write!(fmt, "invalid unit at offset {}", at),
        }
    }
}

impl ::std::error::Error for WcsToMbError {
    fn description(&self) -> &str {
        match *self {
            WcsToMbError::InvalidAt(_) => "invalid unit",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MbsToUniError {
    InvalidAt(usize),
    Incomplete,
    OutOfBufferAt(usize),
}

impl From<MbsToWcError> for MbsToUniError {
    fn from(v: MbsToWcError) -> Self {
        match v {
            MbsToWcError::InvalidAt(at) => MbsToUniError::InvalidAt(at),
            MbsToWcError::Incomplete => MbsToUniError::Incomplete,
            MbsToWcError::OutOfBufferAt(at) => MbsToUniError::OutOfBufferAt(at),
        }
    }
}

impl From<WcToUniError> for MbsToUniError {
    fn from(v: WcToUniError) -> Self {
        match v {
            WcToUniError::InvalidAt(at) => MbsToUniError::InvalidAt(at),
            WcToUniError::Incomplete => MbsToUniError::Incomplete,
        }
    }
}

fn map_err<T, E, F>(v: Result<T, E>) -> Result<T, F> where E: Into<F> {
    v.map_err(Into::into)
}

impl fmt::Display for MbsToUniError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MbsToUniError::InvalidAt(at) => write!(fmt, "invalid unit at offset {}", at),
            MbsToUniError::Incomplete => write!(fmt, "incomplete unit"),
            MbsToUniError::OutOfBufferAt(at) => write!(fmt, "character too large to transcode at offset {}", at),
        }
    }
}

impl ::std::error::Error for MbsToUniError {
    fn description(&self) -> &str {
        match *self {
            MbsToUniError::InvalidAt(_) => "invalid unit",
            MbsToUniError::Incomplete => "incomplete unit",
            MbsToUniError::OutOfBufferAt(_) => "character too large to transcode",
        }
    }
}
