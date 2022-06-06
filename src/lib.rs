use std::ffi::CStr;
use std::os::raw;

pub mod device;
mod event;
pub mod monitor;

// todo: document that device monitoring requires libudev

// FFI utility functions
pub(crate) fn to_owned_str(raw: *const raw::c_char) -> String {
    unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() }
}
