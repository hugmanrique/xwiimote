use crate::device::Address;
use crate::to_owned_str;
use std::alloc::{dealloc, Layout};
use std::os::raw;
use std::path::PathBuf;

/// An iterator over the addresses of all connected Wii Remote
/// devices, and optionally, over newly-connected devices (using
/// a polling mechanism).
pub struct Monitor {
    inner: *mut xwiimote_sys::monitor,
    discover: bool,
    // Have we finished returning the connected devices?
    enumerated: bool,
}

impl Monitor {
    /// Iterates over the addresses of all connected devices.
    pub fn enumerate() -> Self {
        Self::new(false)
    }

    /// First iterates over the addresses of all connected devices,
    /// and then over the addresses of hot-plugged devices.
    pub fn discover() -> Self {
        Self::new(true)
    }

    fn new(poll: bool) -> Self {
        // Create a new monitor based on udevd events.
        // todo: allow using the monitor with kernel uevents
        let inner = unsafe { xwiimote_sys::monitor_new(poll, false) };
        if poll {
            // Enable blocking mode
            assert!(unsafe { xwiimote_sys::monitor_get_fd(inner, true) } != -1);
        }
        Self {
            inner,
            discover: poll,
            enumerated: false,
        }
    }
}

impl Iterator for Monitor {
    type Item = Address;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.discover && self.enumerated {
            // Already enumerated all connected devices.
            return None;
        }

        let path_str = unsafe { xwiimote_sys::monitor_poll(self.inner) };
        if path_str.is_null() {
            // After all connected devices are returned, polling
            // returns `null` once.
            self.enumerated = true;
            if self.discover {
                // Poll again and return the first discovered device.
                self.next()
            } else {
                None
            }
        } else {
            let path = PathBuf::from(to_owned_str(path_str));
            unsafe { dealloc(path_str as *mut u8, Layout::new::<raw::c_char>()) };
            Some(Address::from(path))
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        // When created, the monitor has a ref-count of 1.
        // Decrement to zero to destroy the object.
        unsafe { xwiimote_sys::monitor_unref(self.inner) };
    }
}

#[cfg(test)]
mod tests {
    use super::Monitor;

    #[test]
    fn create_monitor() {
        Monitor::enumerate();
    }
}
