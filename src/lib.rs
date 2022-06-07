//! This library provides a Rust interface to the [xwiimote] user-space
//! library.
//!
//! The following functionality is provided:
//! - [Device discovery](monitor)
//! - [Device connection](device)
//!     - Querying of device kind, connected extension, LED lights,
//!       battery level, rumble motor, etc.
//!     - Open, close and detect available [interfaces](device::Channels).
//!     - Efficient [event dispatching](event) through `epoll`.
//!
//! # Examples
//! [xwiimote]: https://github.com/dvdhrm/xwiimote

use std::ffi::CStr;
use std::os::raw;

pub mod device;
pub mod event;
pub mod key;
pub mod monitor;

// todo: document that device monitoring requires libudev

// FFI utility functions
pub(crate) fn to_owned_str(raw: *const raw::c_char) -> String {
    unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() }
}
