/*!
Encoding conversion support.
*/
use std::fmt;

pub mod mb_x_wc;

#[cfg(target_os="linux")]
pub mod linux;

#[cfg(target_os="linux")]
pub use self::linux as os;

#[cfg(target_os="windows")]
pub mod windows;

#[cfg(target_os="windows")]
pub use self::windows as os;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WcToUniError {
    InvalidAt(usize),
    Incomplete,
}

impl fmt::Display for WcToUniError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WcToUniError::InvalidAt(at) => write!(fmt, "invalid unit at offset {}", at),
            WcToUniError::Incomplete => write!(fmt, "incomplete unit"),
        }
    }
}

impl ::std::error::Error for WcToUniError {
    fn description(&self) -> &str {
        match *self {
            WcToUniError::InvalidAt(_) => "invalid unit",
            WcToUniError::Incomplete => "incomplete unit",
        }
    }
}
