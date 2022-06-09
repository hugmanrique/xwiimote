//! This library provides a simple and safe Rust interface to
//! the [xwiimote] user-space library.
//!
//! At a high level, it provides:
//! - [Device enumeration and discovery](Monitor)
//! - [Device connection](Device)
//!    - Query the device kind, extension data, LED lights,
//!      battery level, rumble motor, etc.
//!    - Open, close and detect available [channels](Channels).
//!    - Efficient [event dispatching](Device::events) through `epoll`.
//!
//! [xwiimote]: https://github.com/dvdhrm/xwiimote
//! [tokio]: https://crates.io/crates/tokio
// todo: add examples and fix links
use crate::event::EventStream;
use crate::io_blocker::IoBlocker;
use bitflags::bitflags;
use futures::Stream;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::pin::Pin;

use std::task::Poll;
use std::time::Duration;
use std::{alloc, io, ptr, thread};

pub mod event;
mod io_blocker;

// FFI and libc utilities

macro_rules! bail_if {
    ($e:expr) => {
        if $e {
            return Err(std::io::Error::last_os_error());
        }
    };
}

// Expose macro to all modules within crate.
pub(crate) use bail_if;

/// Converts a C string into a Rust [`String`](std::String).
fn into_owned_str(raw: *const libc::c_char) -> String {
    unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() }
}

fn dealloc_str(str: *const libc::c_char) {
    unsafe { alloc::dealloc(str as *mut u8, alloc::Layout::new::<libc::c_char>()) };
}

pub(crate) type Result<T> = io::Result<T>;

/// A Wii Remote device address.
#[derive(Clone, Debug)]
pub struct Address(PathBuf);

impl Address {
    /// Converts the path given as a C string to an address.
    fn from_raw(path_str: *const libc::c_char) -> Self {
        let path = PathBuf::from(into_owned_str(path_str));
        path.into()
    }
}

impl From<PathBuf> for Address {
    /// Creates the device address at the specified path.
    ///
    /// If the file at the path exists, it should represent the root
    /// note of a Wii Remote device.
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

// Device monitoring (enumeration and discovery)

/// Enumerates the addresses of connected Wii Remotes and optionally
/// streams device addresses as new devices are discovered. An address
/// may be returned multiple times.
///
/// The stream returns `None` only if discover is disabled and all
/// connected devices have been returned.
///
/// A monitor should be dropped when no longer needed to avoid
/// needlessly polling the system for new devices.
pub struct Monitor {
    handle: *mut xwiimote_sys::monitor,
    // The file descriptor used by the handle monitor, only present
    // in discovery mode to monitor for hot-plug events.
    fd: Option<RawFd>,
    // Have we returned all the connected devices?
    enumerated: bool,
}

impl Monitor {
    const HOTPLUG_EVENTS: libc::c_int = libc::EPOLLIN | libc::EPOLLHUP | libc::EPOLLPRI;

    /// Creates a monitor that first streams the connected devices' addresses
    /// and, if `discover` is `true`, then listens for hot-plug events,
    /// streaming the new addresses.
    pub fn new(discover: bool) -> Result<Self> {
        // Create monitor based on udevd events.
        let handle = unsafe { xwiimote_sys::monitor_new(discover, false) };
        bail_if!(handle.is_null());

        Ok(Monitor {
            handle,
            fd: discover.then(|| unsafe { xwiimote_sys::monitor_get_fd(handle, false) }),
            enumerated: false,
        })
    }
}

impl Stream for Monitor {
    type Item = Result<Address>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let raw_path = if self.enumerated {
            // Discover devices only if `self.fd` is present. Otherwise,
            // we completed the enumeration process.
            let fd = match self.fd {
                Some(fd) => fd,
                None => return Poll::Ready(None),
            };

            let raw_path = unsafe { xwiimote_sys::monitor_poll(self.handle) };
            if raw_path.is_null() {
                // No new device is available, arrange for `wake` to be called
                // once a new device is found.
                IoBlocker::get().set_callback(fd, cx.waker().clone());
                return Poll::Pending;
            }
            raw_path
        } else {
            // Device enumeration requires no blocking, read directly.
            let raw_path = unsafe { xwiimote_sys::monitor_poll(self.handle) };
            if raw_path.is_null() {
                // Read the first `null` address; completed device enumeration.
                self.enumerated = true;

                return if let Some(fd) = self.fd {
                    // Listen for hot-plug events on the monitor descriptor.
                    IoBlocker::get().add_interest(fd, Self::HOTPLUG_EVENTS)?;
                    // Poll again to return the first discovered device.
                    self.poll_next(cx)
                } else {
                    Poll::Ready(None)
                };
            }
            raw_path
        };

        let address = Address::from_raw(raw_path);
        dealloc_str(raw_path);
        Poll::Ready(Some(Ok(address)))
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        if let Some(fd) = self.fd {
            IoBlocker::get()
                .remove_interest(fd, Self::HOTPLUG_EVENTS)
                .expect("failed to remove interest for monitor fd");
        }
        // Decrements ref-count to zero. This closes `self.fd`, if set.
        unsafe { xwiimote_sys::monitor_unref(self.handle) };
    }
}

// Device and interfaces

bitflags! {
    /// Represents the channels that can be opened on a [`Device`].
    ///
    /// The `xwiimote` library calls these interfaces.
    pub struct Channels: libc::c_uint {
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

/// A connected Wii Remote.
pub struct Device {
    pub(crate) handle: *mut xwiimote_sys::iface,
    // Have we opened the core channel in writable mode? We keep track
    // of this because some operations like `rumble` need this channel
    // open to function.
    core_open: bool,
}

impl Device {
    /// Connects to the Wii Remote at the given address.
    pub fn connect(address: &Address) -> Result<Self> {
        let mut handle = ptr::null_mut();
        let path = CString::new(address.0.as_os_str().as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(500));

        let res_code = unsafe { xwiimote_sys::iface_new(&mut handle, path.as_ptr()) };
        bail_if!(res_code != 0);

        // Watch the device for hot-plug events. Otherwise, the
        // `xwiimote_sys:iface_dispatch` function does not report
        // events of type `xwii_sys::EVENT_GONE`, which we need to
        // remove interest for the device file in the `IoBlocker`
        // (see `EventStream::remove_interest`).
        let res_code = unsafe { xwiimote_sys::iface_watch(handle, true) };
        bail_if!(res_code != 0);

        Ok(Self {
            handle,
            core_open: false,
        })
    }

    // Channels

    /// Opens the given channels for communication.
    ///
    /// If a given channel is already open, it is ignored. If any channel
    /// fails to open, the function still tries to open the remaining
    /// requested channels and then returns the error.
    ///
    /// A channel may be closed automatically e.g. if an extension is
    /// unplugged or on error conditions.
    pub fn open(&mut self, channels: Channels, writable: bool) -> Result<()> {
        let ifaces = channels.bits | (writable as libc::c_uint) << 16;
        let res_code = unsafe { xwiimote_sys::iface_open(self.handle, ifaces) };
        bail_if!(res_code != 0);

        if channels.contains(Channels::CORE) && writable {
            self.core_open = true;
        }
        Ok(())
    }

    fn ensure_core_open(&mut self) -> Result<()> {
        if !self.core_open {
            self.open(Channels::CORE, true)?
        }
        Ok(())
    }

    /// Closes the given channels.
    ///
    /// If a channel is already closed, it is ignored.
    pub fn close(&mut self, channels: Channels) -> Result<()> {
        if channels.contains(Channels::CORE) {
            self.core_open = false;
        }
        unsafe { xwiimote_sys::iface_close(self.handle, channels.bits) };
        Ok(())
    }

    /// Lists the currently open channels.
    pub fn all_open(&self) -> Channels {
        Channels::from_bits(unsafe { xwiimote_sys::iface_opened(self.handle) }).unwrap()
    }

    /// Lists the channels that can be opened, including those
    /// that are already open.
    ///
    /// A channel can become available as a result of an extension being
    /// plugged to the device. Correspondingly, it becomes unavailable
    /// when the extension is disconnected.
    ///
    pub fn available(&self) -> Channels {
        Channels::from_bits(unsafe { xwiimote_sys::iface_available(self.handle) }).unwrap()
    }

    // Events

    /// Returns an stream that yields events received from the device.
    ///
    /// Most event types are received only if the appropriate channels
    /// are open. See [`EventKind`](crate::event::EventKind) for more.
    pub fn events(&self) -> Result<impl Stream<Item = Result<event::Event>> + '_> {
        EventStream::try_new(self)
    }

    // Out-of-band actions (these don't require any channel open to work)

    /// Reads the current state of the LED light.
    pub fn led(&self, light: Led) -> Result<bool> {
        let mut enabled = false;
        let res_code = unsafe {
            xwiimote_sys::iface_get_led(self.handle, light as libc::c_uint, &mut enabled)
        };
        bail_if!(res_code != 0);
        Ok(enabled)
    }

    /// Changes the state of the LED light.
    pub fn set_led(&self, light: Led, enabled: bool) -> Result<()> {
        let res_code =
            unsafe { xwiimote_sys::iface_set_led(self.handle, light as libc::c_uint, enabled) };
        bail_if!(res_code != 0);
        Ok(())
    }

    /// Reads the current battery level.
    ///
    /// # Returns
    /// The battery level as a percentage from 0 to 100%, where 100%
    /// means the battery is fully-charged.
    pub fn battery(&self) -> Result<u8> {
        let mut level = 0;
        let res_code = unsafe { xwiimote_sys::iface_get_battery(self.handle, &mut level) };
        bail_if!(res_code != 0);
        Ok(level)
    }

    /// Returns the device type identifier.
    pub fn kind(&self) -> Result<String> {
        let mut raw_kind = ptr::null_mut();
        let res_code = unsafe { xwiimote_sys::iface_get_devtype(self.handle, &mut raw_kind) };
        bail_if!(res_code != 0);

        let kind = into_owned_str(raw_kind);
        dealloc_str(raw_kind);
        Ok(kind)
    }

    /// Returns the current extension type identifier.
    pub fn extension(&self) -> Result<String> {
        let mut raw_ext_kind = ptr::null_mut();
        let res_code = unsafe { xwiimote_sys::iface_get_extension(self.handle, &mut raw_ext_kind) };
        bail_if!(res_code != 0);

        let ext_kind = into_owned_str(raw_ext_kind);
        dealloc_str(raw_ext_kind);
        Ok(ext_kind)
    }

    /// Toggles the rumble motor.
    ///
    /// If the core channel is closed, it is opened in writable mode.
    pub fn rumble(&mut self, enabled: bool) -> Result<()> {
        self.ensure_core_open()?;
        let res_code = unsafe { xwiimote_sys::iface_rumble(self.handle, enabled) };
        bail_if!(res_code != 0); // the channel might have been closed by the kernel
        Ok(())
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

    /// Updates the Motion Plus sensor normalization values.
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
