use crate::event::Event;
use crate::to_owned_str;
use bitflags::bitflags;
use std::error::Error;
use std::fmt::Formatter;
use std::os::raw;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{fmt, ptr};

#[derive(Debug, Clone)]
pub struct DeviceError(i32);

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "device error: {}", self.0)
    }
}

impl Error for DeviceError {}

type DeviceResult<T> = Result<T, DeviceError>;

/// A Wii Remote device address.
#[derive(Clone, Debug)]
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

bitflags! {
    /// Represents the channels that can be opened on a `Device`.
    pub struct Channels: raw::c_uint {
        // todo: improve docs
        /// Primary channel.
        const CORE = 0x1;
        /// Accelerometer channel.
        const ACCELEROMETER = 0x2;
        /// IR camera channel.
        const IR = 0x4;
        /// MotionPlus extension channel.
        const MOTION_PLUS = 0x100;
        /// Nunchuk extension channel.
        const NUNCHUK = 0x200;
        /// Classic controller channel.
        const CLASSIC_CONTROLLER = 0x400;
        /// Balance board channel.
        const BALANCE_BOARD = 0x800;
        /// ProController channel.
        const PRO_CONTROLLER = 0x1000;
        /// Drums channel.
        const DRUMS = 0x2000;
        /// Guitar channel.
        const GUITAR = 0x4000;
    }
}

/// Motion Plus sensor normalization and calibration values.
///
/// The absolute offsets are subtracted from any Motion Plus
/// sensor data before they are returned in an event.
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct MotionPlusNormalization {
    /// Absolute x-axis offset.
    pub x: i32,
    /// Absolute y-axis offset.
    pub y: i32,
    /// Absolute z-axis offset
    pub z: i32,
    /// Calibration factor used to establish the zero-point of
    /// the Motion Plus sensor data depending on its output.
    pub factor: i32,
}

/// Describes the communication with a device.
pub struct Device {
    handle: *mut xwiimote_sys::iface,
    // Have we opened the core channel in writable mode?
    // We need to keep track of this because some operations
    // like `rumble(bool)` require this channel.
    core_open: bool,
}

impl Device {
    /// Opens the Wii Remote at the given address.
    pub fn new(address: &Address) -> DeviceResult<Self> {
        let mut handle = ptr::null_mut();
        let path = address.0.as_os_str().as_bytes().as_ptr() as *const raw::c_char;

        let err_code = unsafe { xwiimote_sys::iface_new(&mut handle, path) };
        if err_code == 0 {
            Ok(Device {
                handle,
                core_open: false,
            })
        } else {
            Err(DeviceError(err_code))
        }
    }

    // Channels (these are called interfaces in xwiimote)

    // todo: document that opening a channel gives access to more event kinds.
    /// Opens the given channels on the device.
    ///
    /// If a given channel is already open, it is ignored. If any channel
    /// fails to open, this function still tries to open the remaining
    /// requested channels and then returns the error.
    ///
    /// A channel may be closed automatically if the e.g. an extension
    /// is unplugged or on error conditions.
    // todo: return the channels that failed.
    // todo: document that an event is emitted when a channel is closed.
    pub fn open(&mut self, channels: Channels, writable: bool) -> DeviceResult<()> {
        let interfaces = channels.bits | (writable as raw::c_uint) << 16;
        let err_code = unsafe { xwiimote_sys::iface_open(self.handle, interfaces) };
        if err_code == 0 {
            if channels.contains(Channels::CORE) {
                self.core_open = true;
            }
            Ok(())
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Closes the given channels on the device.
    ///
    /// If a channel is already closed, it is ignored.
    pub fn close(&mut self, channels: Channels) {
        if channels.contains(Channels::CORE) {
            self.core_open = false;
        }
        unsafe { xwiimote_sys::iface_close(self.handle, channels.bits) }
    }

    /// Returns the channels that are currently open.
    pub fn all_open(&self) -> Channels {
        Channels::from_bits(unsafe { xwiimote_sys::iface_available(self.handle) }).unwrap()
    }

    /// Returns the channels that can be opened on the device.
    ///
    /// A channel can become available if an extension is plugged
    /// to the device. Similarly, it becomes unavailable when the
    /// extension is disconnected.
    pub fn available(&self) -> Channels {
        Channels::from_bits(unsafe { xwiimote_sys::iface_available(self.handle) }).unwrap()
    }

    /// Watch the device for hot-plug events.
    pub fn watch(&mut self, enabled: bool) -> DeviceResult<()> {
        let err_code = unsafe { xwiimote_sys::iface_watch(self.handle, enabled) };
        if err_code == 0 {
            Ok(())
        } else {
            Err(DeviceError(err_code))
        }
    }

    pub fn events(&mut self) -> impl Iterator<Item = Event> {}

    // Actions on "main" channel (it isn't a channel per se)

    /// Reads the state of the LED light.
    ///
    /// # Returns
    /// On success, `true` if the light is enabled and `false`
    /// if disabled. Otherwise, an error is returned.
    pub fn led(&self, light: Led) -> DeviceResult<bool> {
        let mut enabled = false;
        let err_code =
            unsafe { xwiimote_sys::iface_get_led(self.handle, light as raw::c_uint, &mut enabled) };
        if err_code == 0 {
            Ok(enabled)
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Sets the state of the LED light.
    pub fn set_led(&mut self, light: Led, enabled: bool) -> DeviceResult<()> {
        let err_code =
            unsafe { xwiimote_sys::iface_set_led(self.handle, light as raw::c_uint, enabled) };
        if err_code == 0 {
            Ok(())
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Reads the current battery level.
    ///
    /// # Returns
    /// The battery level as a percentage from 0 to 100, where
    /// 100 means the battery is full.
    pub fn battery(&self) -> DeviceResult<u8> {
        let mut capacity = 0;
        let err_code = unsafe { xwiimote_sys::iface_get_battery(self.handle, &mut capacity) };
        if err_code == 0 {
            Ok(capacity)
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Reads the device type identifier.
    pub fn kind(&self) -> DeviceResult<String> {
        let mut kind = ptr::null_mut();
        let err_code = unsafe { xwiimote_sys::iface_get_devtype(self.handle, &mut kind) };
        if err_code == 0 {
            Ok(to_owned_str(kind))
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Reads the extension type identifier.
    pub fn extension(&self) -> DeviceResult<String> {
        let mut ext_type = ptr::null_mut();
        let err_code = unsafe { xwiimote_sys::iface_get_extension(self.handle, &mut ext_type) };
        if err_code == 0 {
            Ok(to_owned_str(ext_type))
        } else {
            Err(DeviceError(err_code))
        }
    }

    /// Toggles the rumble motor.
    ///
    /// If the core channel is closed, it is opened in writable
    /// mode.
    pub fn rumble(&mut self, enable: bool) -> DeviceResult<()> {
        if !self.core_open {
            self.open(Channels::CORE, true)?;
        }
        let err_code = unsafe { xwiimote_sys::iface_rumble(self.handle, enable) };
        if err_code == 0 {
            Ok(())
        } else {
            // The channel might have been closed by the kernel.
            Err(DeviceError(err_code))
        }
    }

    // Motion Plus sensor normalization

    /// Reads the Motion Plus sensor normalization values.
    pub fn mp_normalization(&self) -> MotionPlusNormalization {
        let mut values = MotionPlusNormalization::default();
        unsafe {
            xwiimote_sys::iface_get_mp_normalization(
                self.handle,
                &mut values.x,
                &mut values.y,
                &mut values.z,
                &mut values.factor,
            )
        };
        values
    }

    /// Sets the Motion Plus sensor normalization values.
    pub fn set_mp_normalization(&mut self, values: &MotionPlusNormalization) {
        unsafe {
            xwiimote_sys::iface_set_mp_normalization(
                self.handle,
                values.x,
                values.y,
                values.z,
                values.factor,
            )
        };
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // Also drops all open interfaces.
        unsafe { xwiimote_sys::iface_unref(self.handle) };
    }
}

/// The Wii Remote LED lights.
#[derive(Copy, Clone, Debug)]
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
