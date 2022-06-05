use crate::monitor::Monitor;
use crate::to_owned_str;
use std::os::raw;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{io, ptr};
use xwiimote_sys::iface;

type Result<T> = io::Result<T>;

/// A Wii Remote device address.
pub struct Address(PathBuf);

impl From<PathBuf> for Address {
    /// Creates a new address with the specified path.
    ///
    /// If the file at the path exists, it should represent the root
    /// node of a Wii Remote device. Otherwise, calling
    /// `Device::new(Address)` will fail.
    fn from(path: PathBuf) -> Self {
        Self(path.clone())
    }
}

impl From<&Path> for Address {
    /// Creates a new address with the specified path.
    ///
    /// If the file at the path exists, it should represent the root
    /// node of a Wii Remote device. Otherwise, calling
    /// `Device::new(Address)` will fail.
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

// todo
enum DeviceType {}

/// Describes the communication with a device.
struct Device(*mut iface);

impl Device {
    /// Opens the Wii Remote at the given address.
    pub fn open(address: &Address) -> Result<Self> {
        let mut inner = ptr::null_mut();
        let path = address.0.as_os_str().as_bytes().as_ptr() as *const raw::c_char;

        let err_code = unsafe { xwiimote_sys::iface_new(&mut inner, path) };
        if err_code == 0 {
            Ok(Device(inner))
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Reads the current battery level.
    ///
    /// # Returns
    /// The battery level as a percentage from 0 to 100, where
    /// 100 means the battery is full.
    pub fn battery(&self) -> Result<u8> {
        let mut capacity = 0;
        let err_code = unsafe { xwiimote_sys::iface_get_battery(self.0, &mut capacity) };
        if err_code == 0 {
            Ok(capacity)
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Reads the device type identifier.
    pub fn kind(&self) -> Result<String> {
        let mut kind = ptr::null_mut();
        let err_code = unsafe { xwiimote_sys::iface_get_devtype(self.0, &mut kind) };
        if err_code == 0 {
            Ok(to_owned_str(kind))
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Reads the extension type identifier.
    pub fn extension(&self) -> Result<String> {
        let mut ext_type = ptr::null_mut();
        let err_code = unsafe { xwiimote_sys::iface_get_extension(self.0, &mut ext_type) };
        if err_code == 0 {
            Ok(to_owned_str(ext_type))
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Reads the state of the LED light.
    ///
    /// # Returns
    /// On success, `true` if the light is enabled and `false`
    /// if disabled. Otherwise, an error is returned.
    pub fn led(&self, light: Led) -> Result<bool> {
        let mut enabled = false;
        let err_code = unsafe { xwiimote_sys::iface_get_led(self.0, light.raw(), &mut enabled) };
        if err_code == 0 {
            Ok(enabled)
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Sets the state of the LED light.
    pub fn set_led(&self, light: Led, enabled: bool) -> Result<()> {
        let err_code = unsafe { xwiimote_sys::iface_set_led(self.0, light.raw(), enabled) };
        if err_code == 0 {
            Ok(())
        } else {
            Err(iface_error(err_code))
        }
    }

    /// Toggles the rumble motor.
    pub fn rumble(&self, enable: bool) -> Result<()> {
        let err_code = unsafe { xwiimote_sys::iface_rumble(self.0, enable) };
        if err_code == 0 {
            Ok(())
        } else {
            Err(iface_error(err_code))
        }
    }

    // todo: MP normalization
}

impl Drop for Device {
    fn drop(&mut self) {
        // Close all interfaces, never fails
        unsafe { xwiimote_sys::iface_close(self.0, raw::c_uint::MAX) };
    }
}

/// The Wii Remote LED lights.
pub enum Led {
    /// The left-most light.
    One = 1,
    /// The mid-left light.
    Two,
    /// The mid-right light.
    Three,
    /// The right-most light.
    Four,
}

impl Led {
    fn raw(self) -> raw::c_uint {
        self as u32
    }
}

fn iface_error(_code: raw::c_int) -> io::Error {
    // todo: see if library actually populates `errno`.
    io::Error::last_os_error()
}

/*fn to_raw_str<T: AsRef<str>>(str: T) -> *const raw::c_char {

}*/
